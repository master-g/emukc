---
title: "Account and profile lifecycle: registration, auth tokens, sessions, and game-state init"
date: 2026-06-22
category: architecture-patterns
module: emukc_gameplay
problem_type: architecture_pattern
component: service_object
severity: high
applies_when:
  - "Implementing registration, sign-in, token auth, or session management"
  - "Creating or wiping profiles and initializing per-profile game state"
tags: [user-lifecycle, authentication, tokens, sessions, profile, accountops, profileops]
related_components: [emukc_db, emukc_crypto]
---

# Account and profile lifecycle: registration, auth tokens, sessions, and game-state init

## Context

Accounts authenticate via username/password and tokens; accounts support
multiple profiles with independent game state. `AccountOps` and `ProfileOps`
(`emukc_gameplay`) govern registration, token issuance/validation, session
management, and per-profile initialization. Migrated from
the retired openspec user-lifecycle capability spec (see `docs/migration/openspec-sunset-log.md`).

## Guidance

### Account registration and authentication

The system SHALL allow account creation with username/password and
authentication via tokens, via `AccountOps`.

- New account registration: valid username (≥ 4 chars) + password (≥ 7 chars)
  → hashed password, access token + refresh token issued, no profiles
  initially.
- Duplicate username → `UsernameTaken`.
- Username < 4 chars → `UsernameTooShort`.
- Password < 7 chars → `PasswordTooShort`.
- Sign in with correct credentials: password verified against stored hash; new
  access/refresh tokens issued; `last_login` updated.
- Sign in with wrong credentials → `InvalidUsernameOrPassword`.
- Valid (non-expired) access token → associated account returned, `last_login`
  updated.
- Valid (non-expired) session token → associated profile returned; session
  token expiry renewed.
- Expired token → `TokenExpired`.
- Logout with an access token: all tokens (access, refresh, session) under the
  same account are deleted.
- Account deletion with correct credentials: all tokens, profiles, and the
  account record are removed.

### Profile management

Accounts SHALL support multiple profiles with independent game state, via
`ProfileOps`.

- New profile creation (authenticated account, unique name) → new profile
  record with a session token; game data initialized via
  `init_profile_game_data` (materials, fleets, ships, quests, etc.).
- Duplicate profile name for the same account → `ProfileExists`.
- Start game session for an existing profile → new session token issued.
- Start session for a profile not belonging to the account → `ProfileNotFound`.
- Select world: `world_id` persisted on the profile record.
- Profile wipe: all profile game data reset via `wipe_profile_game_data`, then
  re-initialized via `init_profile_game_data`; the account and profile record
  itself are preserved.

## Why This Matters

Token/session lifecycle is the security boundary. The access-vs-session-token
distinction (account vs profile) is what lets one account hold multiple player
instances. `init_profile_game_data` is the single chokepoint that ensures every
new or wiped profile starts from a consistent baseline.

## When to Apply

- When implementing auth middleware or token validation.
- When adding a new profile-scoped entity that must be initialized/wiped.
- When changing password hashing or token issuance.

## Examples

- Registration with `"ab"` (2 chars) → `UsernameTooShort`.
- Sign-in with a valid access token renews `last_login`; an expired one →
  `TokenExpired`.
- Profile wipe preserves the account/profile record but re-runs
  `init_profile_game_data`.

## Related

- `docs/solutions/architecture-patterns/material.md` — initial materials are
  created by `init_profile_game_data`.
- `docs/solutions/architecture-patterns/fleet.md` — initial fleet slot 1 is
  unlocked during init.
