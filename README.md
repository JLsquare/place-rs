# place-info [![Rust](https://github.com/JLsquare/place-info/actions/workflows/rust.yml/badge.svg)](https://github.com/JLsquare/place-info/actions/workflows/rust.yml)

A lightweight r/place clone built with Rust for my university.
**Demo**: [https://place.jlsquare.fr/](https://place.jlsquare.fr/)

![place-info screenshot](https://i.imgur.com/vMYIkgD.png)

## Prerequisites

Before you begin, ensure you have the following:

- [Rust](https://www.rust-lang.org/) and Cargo installed.
- An environment setup capable of running SQLite.
- Access to an SMTP server for email functionalities (e.g., smtp.office365.com).

## Setup

1. **Clone the Repository**
    
    ```
    git clone https://github.com/JLsquare/place-info.git
    cd place-info
    ```

2. **Setup Environment Variables**
    
    The project relies on several environment variables to function correctly. We've provided a `.env.example` template with all the necessary keys.
        
    `cp .env.example .env`
    
    Now, edit the `.env` file using your preferred text editor, filling in the necessary details.

3. **Build and Run**
    
    To build and run the project in release mode, execute:
        
    `cargo run --release`
    

## Usage

- Navigate to the website; without changes to the `.env` file, it's `localhost:8080/`.
- Signup with a valid UBS email (this restriction can be modified or removed in the code).
- Verify the email.
- Login.
- Select a pixel and draw.

## Contributing

Pull requests are welcome. For major changes, please open an issue first to discuss what you'd like to change.

## License

[MIT](./LICENSE)
