# About microhttp-rs

## Project Overview
microhttp-rs is a lightweight, efficient HTTP parser library written in Rust. It's designed to provide a simple yet powerful way to parse HTTP requests with a focus on performance, correctness, and ease of use.

## Purpose
The primary purpose of microhttp-rs is to offer a minimalist HTTP parsing solution that can be easily integrated into Rust applications that need to handle HTTP requests, such as web servers, API clients, or network tools. By focusing on the core parsing functionality without unnecessary dependencies, microhttp-rs aims to be both lightweight and reliable.

## Key Design Principles
- **Simplicity**: The API is designed to be intuitive and easy to use, with clear error messages and straightforward function calls.
- **Performance**: The parser is optimized for speed and memory efficiency, making it suitable for high-performance applications.
- **Correctness**: Extensive test coverage ensures that the parser correctly handles a wide range of HTTP requests, including edge cases.
- **Minimal Dependencies**: The core parsing functionality has no external dependencies, making it lightweight and easy to integrate.

## Features
- Parse HTTP requests from byte slices with a simple API
- Support for all common HTTP methods (GET, POST, PUT, DELETE, HEAD, OPTIONS, PATCH)
- Support for HTTP versions 1.0, 1.1, and 2.0
- Case-insensitive header handling
- Proper error handling with descriptive error messages
- High test coverage (>98%) ensuring reliability

## Project Status
microhttp-rs is actively maintained and developed. It's currently in version 0.1.0, which provides all the core functionality for parsing HTTP requests. Future versions will add more features while maintaining the library's focus on simplicity and performance.

## Contributing
Contributions to microhttp-rs are welcome! Whether it's bug reports, feature requests, or code contributions, all forms of help are appreciated. The project follows standard Rust coding practices and has a comprehensive test suite to ensure quality.

## License
microhttp-rs is licensed under the MIT License, making it freely available for both personal and commercial use.