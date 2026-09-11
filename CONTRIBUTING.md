# Contributing to FrostByte

Thank you for your interest in contributing to **FrostByte**! As an open-source project dedicated to keeping Windows machines cool, silent, and safe from runaway processes, we welcome contributions of all kinds: bug fixes, documentation improvements, new features, and feedback.

---

## 🧭 Code of Conduct

By participating in this project, you agree to abide by our [Code of Conduct](CODE_OF_CONDUCT.md). Please treat all contributors and users with respect and empathy.

---

## 🛠️ Development Setup

### Prerequisites
* **Operating System:** Windows 10 (Build 19041+) or Windows 11
* **Rust Toolchain:** Latest stable Rust (`x86_64-pc-windows-msvc`) via [rustup.rs](https://rustup.rs/)
* **Build Tools:** [Visual Studio 2022 C++ Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) with the Windows 10/11 SDK installed.

### Clone and Build

```powershell
git clone https://github.com/your-username/frostbyte.git
cd frostbyte

# Build all workspace packages
cargo build --workspace

# Run all unit tests
cargo test --workspace
```

---

## 🏗️ Architecture Guidelines

Before writing code, please review the documents in the [`docs/`](docs/) directory:
* [Architecture Specification](docs/ARCHITECTURE.md)
* [Safety Rules & Immunity Whitelist](docs/SAFETY_RULES.md)

### Key Rules
1. **Safety First:** Never remove or weaken the hardcoded immune process list in `crates/frostbyte-core/src/safety/whitelist.rs`. Any changes to system immunity or process termination require explicit review.
2. **Minimal Overhead:** FrostByte is designed to save CPU and battery. Avoid adding heavy dependencies, polling loops faster than 1 second, or large memory allocations.
3. **Non-Destructive Defaults:** Always prefer soft-taming (Windows Job Object CPU rate limiting) over process termination.

---

## 🧪 Testing Your Changes

Ensure all tests pass and code is formatted before submitting a pull request:

```powershell
# Format code
cargo fmt --all

# Run linter
cargo clippy --workspace --all-targets -- -D warnings

# Run tests
cargo test --workspace
```

---

## 🚀 Submitting a Pull Request

1. Fork the repository and create your feature branch: `git checkout -b feature/amazing-feature`.
2. Commit your changes with clear, descriptive commit messages.
3. Push to your fork: `git push origin feature/amazing-feature`.
4. Open a Pull Request against the `main` branch.
5. Provide a summary of the problem, your solution, and any testing performed.
