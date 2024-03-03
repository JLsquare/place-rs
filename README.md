# place-rs [![Rust](https://github.com/JLsquare/place-rs/actions/workflows/rust.yml/badge.svg)](https://github.com/JLsquare/place-rs/actions/workflows/rust.yml)

A lightweight r/place clone built with Rust for my university.
**Demo**: [https://place.jlsquare.fr/](https://place.jlsquare.fr/)

![place-rs screenshot](https://i.imgur.com/vMYIkgD.png)

## Prerequisites

Before you begin, ensure you have the following:

- [Rust](https://www.rust-lang.org/) and Cargo installed if you're not using docker.
- An environment setup capable of running SQLite if you're not using docker.
- Access to an SMTP server for email functionalities (e.g., smtp.office365.com).

## Setup

### Without docker

1. **Clone the Repository**
    
    ```
    git clone https://github.com/JLsquare/place-rs.git
    cd place-rs
    ```

2. **Setup Environment Variables**
    
    The project relies on several environment variables to function correctly. We've provided a `.env.example` template with all the necessary keys.
        
    `cp .env.example .env`
    
    Now, edit the `.env` file using your preferred text editor, filling in the necessary details.

3. **Build and Run**
    
    To build and run the project in release mode, execute:
        
    `cargo run --release`

### With docker-compose

1. **Clone the Repository**
    
    ```
    git clone https://github.com/JLsquare/place-rs.git
    cd place-rs
    ```

2. **Setup Environment Variables**
    
    The project relies on several environment variables to function correctly. We've provided a `.env.docker-example` template with all the necessary keys.
        
    `cp .env.example .env`
    
    Now, edit the `.env` file using your preferred text editor, filling in the necessary details.

3. **Edit docker-compose**

    To change the port used by the service

3. **Build and Run**
    
    Build and start
    ```
    docker-compose up
    ```

    Stop
    ```
    docker-compose stop
    ```

    Start
    ```
    docker-compose start
    ```    

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
