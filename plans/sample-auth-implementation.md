# Sample Implementation Plan for Dumb Critic Experiment

This is a sample plan with intentionally seeded defects for testing the experiment framework.

## Overview

Implement a user authentication system for the web application.

## Prerequisites

<!-- DEFECT: type=missing_prerequisite, description=Database connection pool not configured before implementing auth -->
1. Set up the web server framework

## Implementation Steps

<!-- DEFECT: type=wrong_ordering, description=Password hashing should be designed before implementing login -->
1. Create login endpoint
2. Design password hashing strategy
3. Create registration endpoint
4. Set up session management

## Acceptance Criteria

- Users can log in successfully
- <!-- DEFECT: type=ambiguous_acceptance_criteria, description="Secure" is not defined; no specific metrics given -->
- Passwords are stored securely
- <!-- DEFECT: type=stale_reference, description=References old auth library v1.x when v2.x is current -->
- Uses terraphim-auth v1.x for token generation

## Error Handling

<!-- DEFECT: type=missing_rollback, description=No strategy for handling failed login attempts or account lockout -->
- Return 401 for invalid credentials

## Scope

<!-- DEFECT: type=scope_creep, description=OAuth integration is out of stated scope but included in steps -->
- Basic email/password auth only

Implementation includes:
- Email/password login
- Session management
- OAuth integration with Google and GitHub
- Two-factor authentication
- Password reset via SMS

## Dependencies

- web-framework 2.0
- <!-- DEFECT: type=stale_reference, description=bcrypt 0.9 is outdated; 0.15 is current -->
- bcrypt 0.9

## Testing

<!-- DEFECT: type=contradictory_statements, description=Says "all edge cases" covered but only lists happy path -->
- Test all edge cases:
  - Valid login succeeds
  - Invalid password returns error

## Deployment

Deploy to production after code review.
