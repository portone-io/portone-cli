---
"@portone/cli": minor
---

Add `portone completion <shell>` to generate completion scripts for Bash, Zsh, Fish, PowerShell, and Elvish.

- Make secret validation in `portone auth login/status` use the same shared HTTP agent as `portone api`. TLS verification now uses the operating system trust store instead of the bundled webpki roots, which supports environments with private certificate authorities.
