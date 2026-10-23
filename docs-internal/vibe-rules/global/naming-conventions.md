# Global Naming Conventions

## General Principles
#naming #conventions #readability #clarity

Clear, descriptive names are more important than brevity. Code is read far more often than written.

### Variables and Functions

**Good Examples:**
```rust
// Rust
let user_session_timeout = Duration::from_secs(3600);
async fn fetch_user_by_email(email: &str) -> Result<User>

// Python
user_session_timeout = 3600
async def fetch_user_by_email(email: str) -> User

// TypeScript
const userSessionTimeout = 3600;
async function fetchUserByEmail(email: string): Promise<User>
```

**Bad Examples:**
```rust
let ust = Duration::from_secs(3600);  // Too abbreviated
async fn get(e: &str) -> Result<User>  // Non-descriptive
```

**Rules:**
1. Use full words, not abbreviations (except widely known: `id`, `url`, `html`)
2. Variables: noun or noun phrase
3. Functions: verb phrase indicating action
4. Booleans: predicate form (`is_valid`, `has_permission`, `can_edit`)

---

## Language-Specific Conventions

### Rust
#rust #naming #snake-case #kebab-case

- **Variables/Functions**: `snake_case`
- **Types/Traits/Enums**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Crate names**: `kebab-case`
- **Lifetimes**: short lowercase (`'a`, `'static`)

```rust
// Correct
const MAX_CONNECTIONS: usize = 100;
struct UserAccount { }
trait Serialize { }
enum HttpMethod { }
fn calculate_total_price() -> f64 { }

// Incorrect
const max_connections: usize = 100;  // Should be SCREAMING_SNAKE_CASE
struct userAccount { }  // Should be PascalCase
fn CalculateTotalPrice() -> f64 { }  // Should be snake_case
```

### Python
#python #naming #pep8

- **Variables/Functions**: `snake_case`
- **Classes**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE`
- **Private**: prefix with `_`
- **Magic methods**: `__double_underscore__`

```python
# Correct
MAX_RETRIES = 3
class UserSession:
    def _validate_token(self):
        pass

def fetch_user_data(user_id: int) -> dict:
    pass

# Incorrect
class user_session:  # Should be PascalCase
def FetchUserData(userId):  # Should be snake_case, snake_case param
```

### TypeScript
#typescript #naming #camelcase

- **Variables/Functions**: `camelCase`
- **Classes/Interfaces/Types**: `PascalCase`
- **Constants**: `SCREAMING_SNAKE_CASE` or `camelCase`
- **Private**: prefix with `#` or `_`
- **Type parameters**: Single uppercase letter or `PascalCase`

```typescript
// Correct
const MAX_RETRIES = 3;
interface UserSession { }
class UserManager { }
type ResponseData<T> = { }
function fetchUserData(userId: number): Promise<User> { }

// Incorrect
function FetchUserData(user_id: number) { }  // Should be camelCase
interface userSession { }  // Should be PascalCase
```

---

## Boolean Naming
#boolean #naming #predicates

Boolean variables and functions should read like questions or assertions.

**Good Examples:**
```rust
// Rust
let is_authenticated: bool;
let has_permission: bool;
let can_edit: bool;
let should_retry: bool;
fn is_valid_email(email: &str) -> bool

// Python
is_authenticated: bool
has_permission: bool
can_edit: bool
should_retry: bool
def is_valid_email(email: str) -> bool

// TypeScript
let isAuthenticated: boolean;
let hasPermission: boolean;
let canEdit: boolean;
let shouldRetry: boolean;
function isValidEmail(email: string): boolean
```

**Bad Examples:**
```rust
let authenticated: bool;  // Ambiguous: state or action?
let valid: bool;  // Valid what?
fn check_email(email: &str) -> bool  // What is being checked?
```

**Prefixes:**
- `is_` / `is`: State check (is_empty, is_active)
- `has_` / `has`: Possession (has_permission, has_data)
- `can_` / `can`: Capability (can_edit, can_delete)
- `should_` / `should`: Recommendation (should_retry, should_cache)
- `will_` / `will`: Future action (will_expire, will_redirect)

---

## Collection Naming
#collections #naming #plurality

Use plural nouns for collections, singular for items.

**Good Examples:**
```rust
// Rust
let users: Vec<User>;
let active_sessions: HashMap<String, Session>;
for user in users.iter() { }

// Python
users: list[User]
active_sessions: dict[str, Session]
for user in users:

// TypeScript
const users: User[];
const activeSessions: Map<string, Session>;
for (const user of users)
```

**Bad Examples:**
```rust
let user_list: Vec<User>;  // Redundant _list suffix
let user: Vec<User>;  // Confusing singular for collection
```

---

## Function Naming by Purpose
#functions #naming #intent

Name functions by what they do, not how they do it.

### Query Functions (Read-only)
#query #getter #accessor

```rust
// Good
fn get_user(id: u64) -> Option<User>
fn find_user_by_email(email: &str) -> Option<User>
fn list_active_users() -> Vec<User>
fn count_pending_orders() -> usize

// Bad
fn user(id: u64)  // Missing verb
fn do_user_lookup(id: u64)  // "do" is not descriptive
```

### Command Functions (Mutating)
#command #mutation #setter

```rust
// Good
fn create_user(data: UserData) -> Result<User>
fn update_user(id: u64, data: UserData) -> Result<()>
fn delete_user(id: u64) -> Result<()>
fn set_active_status(id: u64, active: bool) -> Result<()>

// Bad
fn user_creation(data: UserData)  // Noun form
fn change_user(id: u64, data: UserData)  // Vague verb
```

### Conversion Functions
#conversion #transformation

```rust
// Good
fn to_json(&self) -> String
fn from_json(json: &str) -> Result<Self>
fn into_dto(self) -> UserDto
fn as_str(&self) -> &str

// Bad
fn json(&self) -> String  // Missing verb
fn convert_to_string(&self) -> String  // Verbose when "to_" is standard
```

### Validation Functions
#validation #checking

```rust
// Good
fn validate_email(email: &str) -> Result<()>
fn is_valid_password(password: &str) -> bool
fn check_permission(user: &User, resource: &str) -> Result<()>

// Bad
fn email_validation(email: &str)  // Noun form
fn password(password: &str) -> bool  // Missing verb
```

---

## Constants and Configuration
#constants #config #magic-numbers

Give meaningful names to magic numbers and configuration values.

**Good Examples:**
```rust
// Rust
const DEFAULT_TIMEOUT_SECS: u64 = 30;
const MAX_RETRY_ATTEMPTS: usize = 3;
const CACHE_TTL_MINUTES: u64 = 15;
const MIN_PASSWORD_LENGTH: usize = 8;

// Python
DEFAULT_TIMEOUT_SECS = 30
MAX_RETRY_ATTEMPTS = 3
CACHE_TTL_MINUTES = 15
MIN_PASSWORD_LENGTH = 8

// TypeScript
const DEFAULT_TIMEOUT_SECS = 30;
const MAX_RETRY_ATTEMPTS = 3;
const CACHE_TTL_MINUTES = 15;
const MIN_PASSWORD_LENGTH = 8;
```

**Bad Examples:**
```rust
const TIMEOUT: u64 = 30;  // Timeout for what? What unit?
const MAX: usize = 3;  // Max what?
const FIFTEEN: u64 = 15;  // Why is this a constant?
```

**Units in Names:**
- Include units when ambiguous: `_secs`, `_ms`, `_bytes`, `_mb`
- Use standard abbreviations: `ttl`, `max`, `min`, `avg`

---

## Type Parameters and Generics
#generics #type-parameters #templates

**Rust:**
```rust
// Single letter for simple cases
struct Vec<T> { }
fn map<T, U>(value: T, f: impl Fn(T) -> U) -> U

// Descriptive for complex cases
struct Cache<Key, Value, Storage> { }
trait Repository<Entity, Id> { }
```

**TypeScript:**
```typescript
// Single letter for simple cases
interface Array<T> { }
function map<T, U>(value: T, f: (val: T) => U): U

// Descriptive for complex cases
interface Repository<TEntity, TId> { }
class Cache<TKey, TValue, TStorage> { }
```

**Convention:**
- Single letters: `T` (type), `E` (element), `K` (key), `V` (value), `R` (result)
- TypeScript prefix: Use `T` prefix for descriptive types (`TEntity`, `TUser`)

---

## Module and File Naming
#modules #files #organization

**Rust:**
- Files: `snake_case.rs`
- Modules: `snake_case`
- Binaries: `kebab-case`

```rust
// Good
src/user_session.rs
src/http_client.rs
bin/terraphim-server

// Bad
src/UserSession.rs
src/HTTPClient.rs
```

**Python:**
- Files: `snake_case.py`
- Packages: `snake_case`

```python
# Good
src/user_session.py
src/http_client.py

# Bad
src/UserSession.py
src/HTTPClient.py
```

**TypeScript:**
- Files: `camelCase.ts` or `kebab-case.ts` (choose one per project)
- Components: `PascalCase.tsx`

```typescript
// Good (if using camelCase)
src/userSession.ts
src/httpClient.ts
src/components/UserProfile.tsx

// Good (if using kebab-case)
src/user-session.ts
src/http-client.ts
src/components/UserProfile.tsx
```

---

## Abbreviations and Acronyms
#abbreviations #acronyms #readability

**Common Acceptable Abbreviations:**
- `id` (identifier)
- `url` (uniform resource locator)
- `uri` (uniform resource identifier)
- `html` (hypertext markup language)
- `json` (JavaScript object notation)
- `api` (application programming interface)
- `db` (database)
- `ttl` (time to live)
- `uuid` (universally unique identifier)

**Acronym Casing:**

```rust
// Rust: Treat acronyms as words
struct HttpClient { }
struct UrlParser { }
fn parse_json_response()

// TypeScript: Similar to Rust
class HttpClient { }
class UrlParser { }
function parseJsonResponse()

// Constants: All caps
const MAX_HTTP_REDIRECTS = 10;
const DEFAULT_API_TIMEOUT = 30;
```

**Avoid:**
```rust
// Bad
struct HTTPClient { }  // Should be HttpClient
fn parseJSONResponse()  // Should be parse_json_response
```

---

## Related Patterns

See also:
- [[documentation-standards]] - Documenting code
- [[code-organization]] - Structuring modules and packages
- [[api-design]] - Naming in public APIs
