use rand::RngCore;

fn main() {
    let mut key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut key);
    let encoded_key = base64::encode(&key);
    
    println!("Generated 256-bit encryption key:");
    println!("{}", encoded_key);
    println!("\nAdd this to your .env file:");
    println!("ENCRYPTION_KEY={}", encoded_key);
}

// To run this script:
// cargo run --bin generate_key
