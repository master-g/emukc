## Purpose
Account and profile lifecycle management for EmuKC. Covers user registration,
authentication (tokens), session management, and per-profile game state initialization.

## Requirements

### Requirement: Account Registration and Authentication
The system SHALL allow users to create accounts with username/password and
authenticate via tokens. Implemented via AccountOps.

#### Scenario: New account registration
- WHEN a user signs up with a valid username (>= 4 characters) and password (>= 7 characters)
- THEN a new account is created with a hashed password
- THEN an access token and a refresh token are issued
- THEN the account has no profiles initially

#### Scenario: Duplicate username rejection
- WHEN a user signs up with a username that already exists
- THEN the operation fails with a UsernameTaken error

#### Scenario: Short username rejection
- WHEN a user signs up with a username shorter than 4 characters
- THEN the operation fails with a UsernameTooShort error

#### Scenario: Short password rejection
- WHEN a user signs up with a password shorter than 7 characters
- THEN the operation fails with a PasswordTooShort error

#### Scenario: Sign in with credentials
- WHEN a user signs in with correct username and password
- THEN the password is verified against the stored hash
- THEN new access and refresh tokens are issued
- THEN the account's last_login timestamp is updated

#### Scenario: Sign in with wrong credentials
- WHEN a user signs in with an incorrect username or password
- THEN the operation fails with an InvalidUsernameOrPassword error

#### Scenario: Authentication via access token
- WHEN a valid (non-expired) access token is presented
- THEN the associated account is returned and last_login is updated

#### Scenario: Authentication via session token
- WHEN a valid (non-expired) session token is presented
- THEN the associated profile is returned
- THEN the session token expiry is renewed

#### Scenario: Expired token rejection
- WHEN an expired token is presented for authentication
- THEN the operation fails with a TokenExpired error

#### Scenario: Logout
- WHEN a user logs out with an access token
- THEN all tokens (access, refresh, session) under the same account are deleted

#### Scenario: Account deletion
- WHEN a user deletes their account with correct credentials
- THEN all tokens, profiles, and the account record are removed

### Requirement: Profile Management
Accounts SHALL support multiple profiles (player game instances), each with
independent game state. Implemented via ProfileOps.

#### Scenario: New profile creation
- WHEN a profile is created for an authenticated account with a unique name
- THEN a new profile record is created with a session token
- THEN the profile's game data is initialized via init_profile_game_data (materials, fleets, ships, quests, etc.)

#### Scenario: Duplicate profile name rejection
- WHEN a profile is created with a name that already exists for the same account
- THEN the operation fails with a ProfileExists error

#### Scenario: Start game session
- WHEN an authenticated user starts a game session for an existing profile
- THEN a new session token is issued for that profile

#### Scenario: Start game with invalid profile
- WHEN a user tries to start a session for a profile that does not belong to their account
- THEN the operation fails with a ProfileNotFound error

#### Scenario: Select world
- WHEN a profile selects a world
- THEN the world_id is persisted on the profile record

#### Scenario: Profile wipe
- WHEN an authenticated user wipes a profile
- THEN all profile game data is reset via wipe_profile_game_data
- THEN game data is re-initialized via init_profile_game_data
- THEN the account and profile record itself are preserved
