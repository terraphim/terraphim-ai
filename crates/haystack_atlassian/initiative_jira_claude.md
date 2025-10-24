Description:
- Extend the existing Juno DevSecOps setup (SU-167) to further solidify deployment practices
- Focus on key areas to ensure robust, secure and repeatable deployments:
  - Updating docker compose files to latest standards
  - Integrating 1password CLI for secure secrets management 
  - Implementing comprehensive end-to-end testing of deployment process
- Goal is to have battle-tested, production-grade deployment for Juno that can be replicated for other projects like Orkus

Acceptance Criteria:
- Docker compose files updated to use latest best practices and standards
- 1password CLI integrated into deployment process for secure handling of secrets (API keys, database credentials, etc)
- End-to-end testing suite covers major deployment steps and scenarios
  - Tests run automatically as part of CI/CD pipeline 
  - Tests cover both happy path and failure/rollback cases
- Deployment documentation updated with latest changes
- Knowledge transfer session held with team to review updates
- Follow-up ticket created to replicate this setup for Orkus (extending SU-158)