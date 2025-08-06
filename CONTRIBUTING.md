# Contributing to Cerberus v5.0

Thank you for your interest in contributing to Cerberus! This document provides guidelines for contributing to the project.

## 🚨 Important Disclaimer

**EXTREME RISK WARNING**: This is experimental trading software that can lose money rapidly. Contributors must understand:

- This software trades with real money and can cause significant financial losses
- Contributors are not liable for any losses incurred by users
- All contributions should prioritize safety and risk management
- Never commit code that could compromise user funds or security

## 🤝 How to Contribute

### Types of Contributions

We welcome the following types of contributions:

1. **Bug Reports** - Help us identify and fix issues
2. **Feature Requests** - Suggest new functionality
3. **Code Contributions** - Submit bug fixes and new features
4. **Documentation** - Improve guides, comments, and examples
5. **Testing** - Help test new features and report issues
6. **Security Reviews** - Audit code for security vulnerabilities

### Getting Started

1. **Fork the repository**
2. **Create a feature branch** (`git checkout -b feature/amazing-feature`)
3. **Make your changes**
4. **Test thoroughly** (see Testing section)
5. **Commit your changes** (`git commit -m 'Add amazing feature'`)
6. **Push to the branch** (`git push origin feature/amazing-feature`)
7. **Open a Pull Request**

## 📋 Development Guidelines

### Code Style

We follow Rust standard conventions:

```bash
# Format code
cargo fmt

# Check for issues
cargo clippy -- -D warnings

# Run tests
cargo test
```

### Commit Messages

Use conventional commit format:

```
type(scope): description

feat(trading): add new momentum strategy
fix(risk): correct position sizing calculation
docs(setup): update installation instructions
test(signals): add unit tests for signal processing
```

Types: `feat`, `fix`, `docs`, `style`, `refactor`, `test`, `chore`

### Code Quality Standards

1. **Safety First**: All code must prioritize user fund safety
2. **Error Handling**: Use proper error handling with `Result<T, E>`
3. **Testing**: Minimum 80% test coverage for new code
4. **Documentation**: All public APIs must be documented
5. **Performance**: Consider performance impact of changes
6. **Security**: Follow security best practices

### Architecture Principles

1. **Modularity**: Keep components loosely coupled
2. **Testability**: Write testable code with dependency injection
3. **Observability**: Add proper logging and metrics
4. **Configurability**: Make behavior configurable
5. **Graceful Degradation**: Handle failures gracefully

## 🧪 Testing Requirements

### Test Categories

1. **Unit Tests**: Test individual functions and modules
2. **Integration Tests**: Test component interactions
3. **End-to-End Tests**: Test complete workflows
4. **Performance Tests**: Ensure performance requirements
5. **Security Tests**: Verify security measures

### Running Tests

```bash
# All tests
cargo test

# Unit tests only
cargo test --lib

# Integration tests
cargo test --test integration

# With coverage
cargo tarpaulin --out Html

# Performance tests
cargo test --release -- --ignored perf_test
```

### Test Requirements for PRs

- [ ] All existing tests pass
- [ ] New functionality has tests
- [ ] Test coverage ≥ 80%
- [ ] Performance tests pass
- [ ] Security tests pass

## 🔒 Security Guidelines

### Security-Critical Areas

1. **Private Key Handling**: Never log or expose private keys
2. **API Keys**: Secure storage and transmission
3. **Database Access**: Prevent SQL injection
4. **Network Requests**: Validate all external data
5. **Error Messages**: Don't leak sensitive information

### Security Checklist

- [ ] No hardcoded secrets
- [ ] Input validation on all external data
- [ ] Proper error handling without information leakage
- [ ] Secure random number generation
- [ ] Constant-time comparisons for sensitive data
- [ ] Memory zeroization for secrets

### Reporting Security Issues

**DO NOT** open public issues for security vulnerabilities.

Instead, email: security@botamelia.com

Include:
- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (if any)

## 📝 Documentation Standards

### Code Documentation

```rust
/// Calculates position size based on portfolio value and risk parameters.
/// 
/// # Arguments
/// 
/// * `portfolio_value` - Current portfolio value in USD
/// * `risk_percent` - Risk percentage (0.0 to 1.0)
/// * `confidence` - Signal confidence (0.0 to 1.0)
/// 
/// # Returns
/// 
/// Position size in USD, capped at maximum allowed
/// 
/// # Examples
/// 
/// ```
/// let size = calculate_position_size(1000.0, 0.02, 0.8);
/// assert!(size <= 1000.0 * 0.5); // Never more than 50%
/// ```
pub fn calculate_position_size(
    portfolio_value: f64,
    risk_percent: f64,
    confidence: f64,
) -> f64 {
    // Implementation...
}
```

### README Updates

When adding new features, update:
- Feature list in README
- Configuration examples
- Usage instructions
- Dependencies if changed

## 🚀 Pull Request Process

### Before Submitting

1. **Test thoroughly** in paper trading mode
2. **Run full test suite** and ensure all pass
3. **Update documentation** for any API changes
4. **Add changelog entry** if user-facing change
5. **Verify security implications**

### PR Template

```markdown
## Description
Brief description of changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
- [ ] Unit tests added/updated
- [ ] Integration tests pass
- [ ] Manual testing completed
- [ ] Paper trading tested

## Security
- [ ] No security implications
- [ ] Security review completed
- [ ] Secrets properly handled

## Checklist
- [ ] Code follows style guidelines
- [ ] Self-review completed
- [ ] Documentation updated
- [ ] Tests added for new functionality
```

### Review Process

1. **Automated Checks**: CI/CD pipeline runs tests
2. **Code Review**: Maintainer reviews code quality
3. **Security Review**: Security-sensitive changes get extra review
4. **Testing**: Changes tested in staging environment
5. **Approval**: Approved by maintainer
6. **Merge**: Squash and merge to main branch

## 🏗️ Development Environment

### Required Tools

```bash
# Rust toolchain
rustup install stable
rustup component add rustfmt clippy

# Development tools
cargo install cargo-watch
cargo install cargo-tarpaulin  # Coverage
cargo install cargo-audit      # Security audit
cargo install cargo-deny       # License/security checks
```

### IDE Setup

#### VS Code Extensions
- rust-analyzer
- CodeLLDB (debugging)
- Better TOML
- GitLens

#### Configuration
```json
{
    "rust-analyzer.checkOnSave.command": "clippy",
    "rust-analyzer.cargo.features": "all"
}
```

### Local Development

```bash
# Watch for changes and rebuild
cargo watch -x check -x test

# Run with debug logging
RUST_LOG=debug cargo run

# Paper trading mode
PAPER_TRADING=true cargo run
```

## 📊 Performance Guidelines

### Performance Requirements

- **Signal Processing**: < 10ms per signal
- **Trade Execution**: < 100ms end-to-end
- **Database Queries**: < 50ms average
- **Memory Usage**: < 512MB under normal load
- **CPU Usage**: < 50% average

### Profiling

```bash
# CPU profiling
cargo install flamegraph
cargo flamegraph --bin cerberus

# Memory profiling
cargo install heaptrack
heaptrack target/release/cerberus
```

## 🐛 Bug Reports

### Bug Report Template

```markdown
**Bug Description**
Clear description of the bug

**To Reproduce**
Steps to reproduce the behavior:
1. Go to '...'
2. Click on '....'
3. See error

**Expected Behavior**
What you expected to happen

**Environment**
- OS: [e.g. Ubuntu 22.04]
- Rust version: [e.g. 1.75.0]
- Cerberus version: [e.g. v5.0.1]

**Logs**
Relevant log output (remove sensitive data)

**Additional Context**
Any other context about the problem
```

### Critical Bugs

For bugs that could cause financial loss:
1. **Immediate notification**: Email security@botamelia.com
2. **Emergency fix**: Create hotfix branch
3. **Fast-track review**: Expedited review process
4. **User notification**: Alert users if necessary

## 🎯 Feature Requests

### Feature Request Template

```markdown
**Feature Description**
Clear description of the feature

**Use Case**
Why is this feature needed?

**Proposed Solution**
How should this feature work?

**Alternatives Considered**
Other solutions you've considered

**Additional Context**
Any other context or screenshots
```

### Feature Development Process

1. **Discussion**: Feature discussed in GitHub Discussions
2. **Design**: Technical design document created
3. **Approval**: Feature approved by maintainers
4. **Implementation**: Feature implemented and tested
5. **Documentation**: Documentation updated
6. **Release**: Feature included in next release

## 📞 Getting Help

- **GitHub Discussions**: General questions and discussions
- **GitHub Issues**: Bug reports and feature requests
- **Telegram**: [@CerberusTrading](https://t.me/CerberusTrading)
- **Email**: support@botamelia.com

## 📄 License

By contributing, you agree that your contributions will be licensed under the MIT License.

## 🙏 Recognition

Contributors will be recognized in:
- CONTRIBUTORS.md file
- Release notes for significant contributions
- GitHub contributor graphs

Thank you for helping make Cerberus better! 🚀
