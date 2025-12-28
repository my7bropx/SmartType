use minimal_ffi::{c_free, c_strlen, to_c_buffer};

fn main() {
    let message = "Hello from Rust via C!";
    println!(
        "Message: '{message}', length via C strlen: {}",
        c_strlen(message)
    );

    let (ptr, len) = to_c_buffer(message.as_bytes());
    if ptr.is_null() {
        eprintln!("Allocation failed");
        return;
    }
    let data = unsafe { std::slice::from_raw_parts(ptr, len) };
    println!("Copied buffer: {} bytes -> {:?}", len, data);

    c_free(ptr);
}
