# RustGram

RustGram is a Telegram-based image hosting service written in Rust. It provides a simple API to upload, retrieve, and get information about images. The service uses a Telegram chat as a storage backend, leveraging the Telegram Bot API to manage images.

## Features

- **Image Upload:** Upload images to a specified Telegram chat.
- **Image Retrieval:** Fetch images using a unique ID.
- **Image Information:** Get metadata about a stored image.
- **Health Check:** Endpoint to monitor the service's health.
- **Rate Limiting:** Middleware to limit the number of requests per minute.
- **CORS:** Configured with a permissive Cross-Origin Resource Sharing policy.
- **Encryption:** Support for encrypting image data before storage.
- **Configuration:** Easily configurable through environment variables.

## Endpoints

- `POST /upload`: Upload a new image.
- `GET /image/:id`: Retrieve an existing image by its ID.
- `GET /info/:id`: Get information about an image by its ID.
- `GET /health`: Check the health of the service.

## Tech Stack

- **Web Framework:** [Axum](https://github.com/tokio-rs/axum)
- **Async Runtime:** [Tokio](https://tokio.rs/)
- **HTTP Client:** [Reqwest](https://github.com/seanmonstar/reqwest)
- **Encryption:** [AES-GCM](https://docs.rs/aes-gcm/latest/aes_gcm/)
- **Image Processing:** [image](https://github.com/image-rs/image)

This project is designed to be a lightweight and efficient solution for self-hosted image storage, utilizing the robustness and availability of Telegram's infrastructure.

## Troubleshooting

- **Missing `ConnectInfo` Extension:** If you encounter an error like "Missing request extension: Extension of type `axum::extract::connect_info::ConnectInfo<core::net::socket_addr::SocketAddr>` was not found," it indicates an issue with the Axum setup not providing connection information. Please ensure your Axum version and server configuration are correct, especially how `axum::serve` is used with `ConnectInfo`.
