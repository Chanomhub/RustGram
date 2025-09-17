use axum::{
    extract::ConnectInfo,
    http::{Request, StatusCode},
    response::Response,
};
use std::{
    collections::HashMap,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct RateLimitLayer {
    requests_per_minute: u32,
    store: Arc<Mutex<HashMap<String, TokenBucket>>>,
}

impl RateLimitLayer {
    pub fn new(requests_per_minute: u32) -> Self {
        Self {
            requests_per_minute,
            store: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl<S> Layer<S> for RateLimitLayer {
    type Service = RateLimitService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RateLimitService {
            inner,
            requests_per_minute: self.requests_per_minute,
            store: self.store.clone(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitService<S> {
    inner: S,
    requests_per_minute: u32,
    store: Arc<Mutex<HashMap<String, TokenBucket>>>,
}

impl<S, B> Service<Request<B>> for RateLimitService<S>
where
    S: Service<Request<B>, Response = Response> + Clone + Send + 'static,
    S::Future: Send + 'static,
    B: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = futures::future::BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<B>) -> Self::Future {
        let mut inner = self.inner.clone();
        let store = self.store.clone();
        let requests_per_minute = self.requests_per_minute;

        Box::pin(async move {
            // Get client IP
            let client_ip = req
                .extensions()
                .get::<ConnectInfo<SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            // Check rate limit
            let allowed = {
                let mut store = store.lock().unwrap();
                let bucket = store
                    .entry(client_ip.clone())
                    .or_insert_with(|| TokenBucket::new(requests_per_minute));
                
                bucket.try_consume()
            };

            if !allowed {
                let response = Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header("Content-Type", "application/json")
                    .body(
                        serde_json::json!({
                            "error": "Rate limit exceeded",
                            "status": 429
                        })
                        .to_string()
                        .into(),
                    )
                    .unwrap();
                return Ok(response);
            }

            // Clean up old entries periodically
            if fastrand::f32() < 0.01 {
                // 1% chance to cleanup
                let mut store = store.lock().unwrap();
                let now = Instant::now();
                store.retain(|_, bucket| now.duration_since(bucket.last_refill) < Duration::from_secs(300));
            }

            inner.call(req).await
        })
    }
}

#[derive(Debug)]
struct TokenBucket {
    tokens: f32,
    capacity: f32,
    refill_rate: f32, // tokens per second
    last_refill: Instant,
}

impl TokenBucket {
    fn new(requests_per_minute: u32) -> Self {
        let capacity = requests_per_minute as f32;
        Self {
            tokens: capacity,
            capacity,
            refill_rate: capacity / 60.0, // convert per-minute to per-second
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self) -> bool {
        self.refill();
        
        if self.tokens >= 1.0 {
            self.tokens -= 1.0;
            true
        } else {
            false
        }
    }

    fn refill(&mut self) {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f32();
        
        self.tokens = (self.tokens + elapsed * self.refill_rate).min(self.capacity);
        self.last_refill = now;
    }
}
