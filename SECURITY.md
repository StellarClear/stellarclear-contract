# Security Policy

## Reporting Security Vulnerabilities

We take the security of the StellarClear protocol seriously. If you discover a security vulnerability or potential exploit within this codebase, please report it responsibly.

### Responsible Disclosure Process

Please report security issues using **GitHub Private Vulnerability Reporting**:

1. Navigate to the repository's **Security** tab on GitHub.
2. Click on **Advisories** and select **Report a vulnerability**.
3. Provide detailed steps to reproduce the issue, including environment details, test cases, and the potential impact.

> [!NOTE]
> Do not disclose security vulnerabilities publicly in issues or discussions until they have been reviewed and addressed.

---

## Security Status

> [!CAUTION]
> **UNAUDITED CODE**: The contracts in this repository are currently in active development and have **NOT** been audited by a third-party security firm. They are provided as-is without warranties of any kind.

---

## Scope

The following areas are in scope for vulnerability reports:
- Authorization bypasses or spoofing in `SettlementRegistry`
- State machine invariant violations (e.g. illegal transitions or re-entering finalized states)
- Cryptographic commitment evasion or collision vulnerabilities
- Persistent storage key collision or corruption
- Replay or griefing attacks in two-party dispute resolution

Out of scope:
- Issues in third-party development toolchains or unreleased Rust compiler versions
- Social engineering attacks
- Bugs or vulnerabilities in off-chain systems outside of this repository
