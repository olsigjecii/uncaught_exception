Of course. While Rust's design with its `Result<T, E>` enum and strict compile-time checks makes traditional "uncaught exceptions" a non-issue, we can simulate the underlying security flaw: mishandling errors caused by unvalidated user input, leading to catastrophic failure and information disclosure. 💥

In Rust, the closest equivalent to an uncaught exception is a `panic!`. A panic occurs when the program encounters an unrecoverable error, unwinds the stack, and typically terminates. A common way to trigger a panic is by calling `.unwrap()` or `.expect()` on a `Result` that holds an `Err` value or an `Option` that is `None`.

This lesson will demonstrate how relying on `unwrap()` with user-controlled data can crash an application and how to properly handle errors to build resilient and secure services.

### **Demonstration Application: Rust & `actix-web`**

Here is a complete, runnable `actix-web` application that contains both the vulnerable code (which will panic) and the fixed, secure code.

#### **Project Structure:**

```
.
├── Cargo.toml
└── src
    └── main.rs
```

#### **`Cargo.toml`**

You will need the following dependencies for our `actix-web` application.

```toml
[package]
name = "uncaught-exception-rust"
version = "0.1.0"
edition = "2021"

[dependencies]
actix-web = "4"
serde = { version = "1.0", features = ["derive"] }
log = "0.4"
env_logger = "0.9"
reqwest = { version = "0.11", features = ["json"] }
thiserror = "1.0"
actix-http = "3"
```

-----

# Uncaught Exception Lesson in Rust

## 📝 Lesson Summary

This project translates the concept of **Uncaught Exceptions** into a Rust context using the `actix-web` framework. While Rust's robust type system prevents traditional uncaught exceptions at compile time, we can simulate the same security risks by mishandling the `Result` type, leading to a `panic`.

The core vulnerability demonstrated is **Information Exposure** caused by improper error handling when processing user-controlled input. An endpoint constructs a backend API URL using the `Host` header from an incoming request.

  * **Vulnerable Path**: The code attempts to parse a URL constructed with the user's `Host` header. [cite\_start]When an invalid `Host` (e.g., an empty string) is provided, the URL parser returns an error.The error handling logic then insecurely reflects the failed URL—including a hardcoded API key—back to the user in the HTTP response.
  * **Secure Path**: The code is remediated using two key strategies:
    1.  **Input Validation**: It checks the `Host` header against a predefined whitelist of allowed domains before using it. [cite: 145]
    2.  **Graceful Error Handling**: It properly handles the `Result` from the URL parsing operation, logging detailed errors internally and returning a generic, safe error message to the user, preventing any information leaks. [cite: 110, 117]

## 🚀 Application Setup

Follow these steps to set up and run the demonstration application.

1.  **Install Rust**: If you don't have Rust installed, get it from [rustup.rs](https://rustup.rs/).

    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    ```

2.  **Clone & Build**: Clone the project and build it using Cargo.

    ```bash
    # Clone this repository (example)
    # git clone <your-repo-url>
    # cd <your-repo-directory>

    # Build the project
    cargo build
    ```

3.  **Run the Application**:

    ```bash
    cargo run
    ```

    The server will start on `http://127.0.0.1:8080`.

## 💥 Demonstrating the Vulnerability

We will send a request with an empty `Host` header to the vulnerable endpoint. The application will fail to parse the URL and return an error message containing the API key.

```bash
# The -H "Host:" flag sends an empty Host header.
curl -v -H "Host: invalid host" "http://127.0.0.1:8080/vulnerable/waitlist?email=attacker@evil.com"
```

### Expected Vulnerable Output

You will receive a `500 Internal Server Error` response. The body of the response will contain the sensitive information, exposing the API key.

```

* Trying 127.0.0.1:8080...
* Connected to 127.0.0.1 (127.0.0.1) port 8080 (#0)
> GET /vulnerable/waitlist?email=attacker@evil.com HTTP/1.1
> User-Agent: curl/7.81.0
> Accept: */*
>
* Mark bundle as not supporting multiuse
< HTTP/1.1 500 Internal Server Error
< content-length: 173
< content-type: text/plain; charset=utf-8
< date: ...
<
Failed to construct backend request. URL: 'https:///v1/waitlist?api_key=88665751-288d-4175-852f-6519d79fdf1f&email=attacker@evil.com', Error: empty host
```

**Success\!** The API key `88665751-288d-4175-852f-6519d79fdf1f` has been leaked.

## ✅ Demonstrating the Mitigation

Now, we'll send the same malicious requests to the secure endpoint.

1.  **Attempt Attack with Empty `Host` Header**:

    The input validation should immediately reject this request.

    ```bash
    curl -v -H "Host:" "http://127.0.0.1:8080/secure/waitlist?email=attacker@evil.com"
    ```

    ### Expected Secure Output (1)

    You will receive a `400 Bad Request` response, and no sensitive information is leaked.

    ```
    < HTTP/1.1 400 Bad Request
    < content-length: 28
    < content-type: text/plain; charset=utf-8
    < date: ...
    <
    Invalid 'Host' header provided.
    ```

2.  **Attempt Attack with Disallowed `Host` Header**:

    The input validation should also reject a host that is not on the whitelist.

    ```bash
    curl -v -H "Host: not-allowed.com" "http://127.0.0.1:8080/secure/waitlist?email=attacker@evil.com"
    ```

    The output will be the same `400 Bad Request` as above.

3.  **Send a Valid Request**:

    Finally, send a request with a valid, whitelisted `Host` header.

    ```bash
    curl -v -H "Host: 127.0.0.1:8080" "http://127.0.0.1:8080/secure/waitlist?email=user@good.com"
    ```

    ### Expected Secure Output (3)

    The request is processed successfully.

    ```
    < HTTP/1.1 200 OK
    < content-length: 71
    < content-type: text/plain; charset=utf-8
    < date: ...
    <
    Thank you for your interest. We will notify you when we are ready to launch.
    ```

