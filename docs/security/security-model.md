# Security Model

P4inz follows defense-in-depth and zero-trust principles.

## Mandatory Controls

- Authentication
- Resource-level authorization
- RBAC
- Input validation
- Rate limiting
- Secret isolation
- Secure sessions
- Webhook verification
- Audit logging
- Data minimization
- Dependency auditing
- Backup and recovery
- AI prompt-injection isolation

## AI

AI cannot:
- grant permissions
- access arbitrary private data
- execute arbitrary shell commands
- execute arbitrary SQL
- perform high-impact administrative actions autonomously

Unknown or unsupported information must not be fabricated.
