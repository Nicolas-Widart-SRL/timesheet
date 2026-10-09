# Hexagonal Architecture in Rust

Notes from [Master Hexagonal Architecture in Rust](https://www.howtocodeit.com/guides/master-hexagonal-architecture-in-rust) (How To Code It). The guide covers Parts I–IV; Part V ("Advanced hexagonal architecture": retries, caches, dead letter queues) is not published yet.

The guide uses an axum + SQLite blogging engine as its example. The principles apply to any application that takes external input or calls the outside world.

## The problem: hard dependencies everywhere

The usual way to write a Rust service ("the Very Bad Application"):

- `main` configures axum middleware, opens an `sqlx::SqlitePool`, and builds the router. It must know the internals of every third-party crate.
- The concrete `SqlitePool` goes into `AppState`, so it flows through every handler.
- The HTTP handler validates the body, opens and commits a transaction, runs SQL, and inspects SQLite error code `2067` to detect a duplicate.

Consequences:

- Changing the HTTP server or the database client means touching `main`, every handler, and every place the concrete type leaked to.
- Handlers cannot be unit tested. They need a real database. Failure paths such as "transaction fails to start" are almost impossible to exercise.
- Integration tests are slow and expensive, so they cannot give exhaustive coverage.

> "The failure to encapsulate and abstract dependencies makes this code hard to maintain."

### Which dependencies may stay hard

Not every dependency needs an abstraction:

- **Keep hard:** crates that are near-language-level or universally adopted, for example `tokio` and `anyhow`. Abstracting them gains nothing. Choose them carefully, because replacing them later is a painful refactor.
- **Abstract:** HTTP frameworks, database clients, message queues, email and metrics providers. Teams replace these regularly (scaling, deprecation, security, a senior engineer's preference).

## Core vocabulary

| Term | Meaning |
| --- | --- |
| **Domain / business logic** | The code that makes the application distinct. HTTP, JSON, and SQL are not special; how you compose them is. |
| **Domain model** | The canonical, always-valid representation of data that the business logic accepts. |
| **Port** | A domain trait. The contract the domain defines for something it needs (outbound, e.g. `AuthorRepository`) or something it offers (inbound, e.g. `AuthorService`). |
| **Adapter** | A concrete implementation of a port (e.g. an SQLite `AuthorRepository`), or a driver that calls the domain (e.g. an HTTP server). |
| **Inbound adapter** | Receives requests from the outside world and calls the domain (HTTP handlers, CLI, RPC). |
| **Outbound adapter** | Implements a port so the domain can reach the outside world (database, email, metrics). |
| **Service** | The trait that declares the business API, plus the domain's implementation of it. Inbound adapters depend on the Service. |
| **Entity** | A uniquely identifiable domain object (e.g. `Author`). Not necessarily one database record. |

Dependencies point in one direction only: **towards the domain**.

## Domain models

```rust
pub struct Author {
    id: Uuid,
    name: AuthorName,
}

/// A validated and formatted name.
pub struct AuthorName(String);

impl AuthorName {
    pub fn new(raw: &str) -> Result<Self, AuthorNameEmptyError> {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            Err(AuthorNameEmptyError)
        } else {
            Ok(Self(trimmed.to_string()))
        }
    }
}

/// The fields required by the domain to create an [Author].
pub struct CreateAuthorRequest {
    name: AuthorName,
}
```

Rules:

- **Validation belongs to the domain, not to request handlers.** Newtype constructors (`AuthorName::new`) are the single source of truth for what is valid. If you cannot build a valid `CreateAuthorRequest`, you cannot call the port.
- **Model "create" requests separately from the entity.** `CreateAuthorRequest` is not `Author { id: Option<Uuid>, .. }`. The data needed to create something and the full representation of it diverge in real applications (for example, a fintech customer profile that fills in over weeks). Keeping them apart prevents a change to the entity from forcing a change to request handling.
- **Be careful with `serde` on domain models.** Derive `Serialize`/`Deserialize` only if the model has no validation (every field value is valid). Otherwise adapters can use Serde to bypass your constructors and create invalid models.

> "Hexagonal architecture is about building for change."

## Errors

```rust
#[derive(Debug, Error)]
pub enum CreateAuthorError {
    #[error("author with name {name} already exists")]
    Duplicate { name: AuthorName },
    #[error(transparent)]
    Unknown(#[from] anyhow::Error),
}
```

- Input validation errors (`AuthorNameEmptyError`) belong to the model constructors, not to the operation error.
- The operation error lists failures from coordinating adapters, in two categories:
  - business rule violations (`Duplicate`);
  - a catch-all `Unknown(anyhow::Error)` for anything the domain cannot handle. `anyhow` keeps a backtrace.
- The enum is a **complete description** of what can go wrong. Callers get compiler-enforced handling of every case.
- In an application, do **not** mark these enums `#[non_exhaustive]`. You want error handling to stop compiling when you add a variant. (A library is different: there, `non_exhaustive` avoids breaking changes.)
- **Do not panic on unexpected errors.** Return `Unknown` instead. Panics poison held mutexes (an `Arc<Mutex<T>>` becomes unusable forever), and other Rust developers do not expect them.
- Add context to opaque errors (`.context("failed to start SQLite transaction")`). When in doubt, give too much information rather than too little.

## Outbound ports: the repository pattern

A repository is "some store of data". The handler should say "give me any store of author data", not "give me SQLite".

```rust
pub trait AuthorRepository: Clone + Send + Sync + 'static {
    /// Asynchronously persist a new [Author].
    ///
    /// # Errors
    ///
    /// - MUST return [CreateAuthorError::Duplicate] if an [Author] with the same
    ///   [AuthorName] already exists.
    fn create_author(
        &self,
        req: &CreateAuthorRequest,
    ) -> impl Future<Output = Result<Author, CreateAuthorError>> + Send;
}
```

Why each bound exists:

- `-> impl Future<..> + Send` instead of `async fn`: a plain `async fn` in a trait does not get a `Send` bound on its future, so it cannot move between threads. That is useless for a multi-threaded web server. Implementations can still be written as `async fn`.
- `Send + Sync`: the repository sits inside an `Arc`, and `Arc<T>` is only `Send`/`Sync` when `T` is `Send + Sync`.
- `'static`: the repository is shared across tasks and must live for the whole program.
- `Clone`: axum requires injected state to be `Clone`.

Document the contract (the `MUST return ...` lines) on the trait. The port is the source of truth for how every implementation behaves.

### The adapter

```rust
#[derive(Debug, Clone)]
pub struct Sqlite {
    pool: sqlx::SqlitePool,
}

impl AuthorRepository for Sqlite {
    async fn create_author(&self, req: &CreateAuthorRequest) -> Result<Author, CreateAuthorError> {
        let mut tx = self.pool.begin().await
            .context("failed to start SQLite transaction")?;

        let author_id = self.save_author(&mut tx, req.name()).await.map_err(|e| {
            if is_unique_constraint_violation(&e) {
                CreateAuthorError::Duplicate { name: req.name().clone() }
            } else {
                anyhow!(e)
                    .context(format!("failed to save author with name {:?}", req.name()))
                    .into()
            }
        })?;

        tx.commit().await.context("failed to commit SQLite transaction")?;
        Ok(Author::new(author_id, req.name().clone()))
    }
}
```

- **Wrap third-party types** (`sqlx::SqlitePool` inside your own `Sqlite`) and expose only what the application needs. This plugs the leaks.
- Transactions, SQL, and database error codes live only in the adapter. The trait says *what*; the adapter decides *how*.
- The adapter translates storage errors into domain errors (`2067` becomes `Duplicate`). It knows nothing about HTTP status codes.
- Constructors that are not part of the port (`Sqlite::new`) can return `anyhow::Result`. At startup the only sensible thing to do with an error is log it.

## Inbound adapters: HTTP

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct CreateAuthorHttpRequestBody {
    name: String,
}

impl CreateAuthorHttpRequestBody {
    fn try_into_domain(self) -> Result<CreateAuthorRequest, AuthorNameEmptyError> {
        let author_name = AuthorName::new(&self.name)?;
        Ok(CreateAuthorRequest::new(author_name))
    }
}

pub async fn create_author<AS: AuthorService>(
    State(state): State<AppState<AS>>,
    Json(body): Json<CreateAuthorHttpRequestBody>,
) -> Result<ApiSuccess<CreateAuthorResponseData>, ApiError> {
    let domain_req = body.try_into_domain()?;
    state
        .author_service
        .create_author(&domain_req)
        .await
        .map_err(ApiError::from)
        .map(|ref author| ApiSuccess::new(StatusCode::CREATED, author.into()))
}
```

- The handler's only job is the request/response cycle: parse transport data, convert it to the domain model, call the domain, map the result back.
- Transport types (`CreateAuthorHttpRequestBody`, `CreateAuthorResponseData`) are separate from domain types. The HTTP API and the domain can now change independently; only the mapping and its tests change.
- **Separate public errors from domain errors.** `ApiError::from(CreateAuthorError)` builds its own message instead of using the domain error's `Display`. Exposing domain errors leaks private details and lets internal changes alter public responses.
  - `Duplicate` maps to `422 Unprocessable Entity`.
  - `Unknown` is logged with its backtrace and maps to a generic `500 Internal Server Error`. The cause is never sent to the client.

> "Always separate your public errors from their domain representations."

## The Service: home of business logic

Real business logic does more than "write to the database and respond 201": record metrics, send a welcome email, publish an event. That logic belongs neither in the handler (transport) nor in the repository (storage). It belongs in a **Service**.

```rust
pub trait AuthorMetrics: Send + Sync + Clone + 'static {
    fn record_creation_success(&self) -> impl Future<Output = ()> + Send;
    fn record_creation_failure(&self) -> impl Future<Output = ()> + Send;
}

pub trait AuthorNotifier: Send + Sync + Clone + 'static {
    fn author_created(&self, author: &Author) -> impl Future<Output = ()> + Send;
}

pub trait AuthorService: Clone + Send + Sync + 'static {
    fn create_author(
        &self,
        req: &CreateAuthorRequest,
    ) -> impl Future<Output = Result<Author, CreateAuthorError>> + Send;
}

#[derive(Debug, Clone)]
pub struct Service<R, M, N>
where
    R: AuthorRepository,
    M: AuthorMetrics,
    N: AuthorNotifier,
{
    repo: R,
    metrics: M,
    notifier: N,
}

impl<R, M, N> AuthorService for Service<R, M, N>
where
    R: AuthorRepository,
    M: AuthorMetrics,
    N: AuthorNotifier,
{
    async fn create_author(&self, req: &CreateAuthorRequest) -> Result<Author, CreateAuthorError> {
        let result = self.repo.create_author(req).await;
        if result.is_err() {
            self.metrics.record_creation_failure().await;
        } else {
            self.metrics.record_creation_success().await;
            self.notifier.author_created(result.as_ref().unwrap()).await;
        }
        result
    }
}
```

- `AppState` holds `Arc<AS: AuthorService>`, not the repository. Handlers no longer know that anything is saved.
- The Service and Repository signatures look alike only because the domain is simple. They are separate traits so they can diverge.
- Ports are generic type parameters (static dispatch), not `dyn` trait objects.
- Whether a port hides the medium (`AuthorNotifier` vs. "email") is a judgement call. If the business requires email, say so in the port.

## Testing strategy

Each layer is tested against mocks of the layer below:

1. **Handlers:** mock the Service. Return each success and error variant and check the HTTP output.
2. **Service:** mock each port (repository, metrics, notifier). Exercise every combination of success and failure.
3. **Integration tests:** run the whole system. Cover happy paths and the most important error scenarios only.

Without a Service, every permutation of repository, metrics, and notifier failures, multiplied by every HTTP parsing failure, must be integration tested.

> "The Service trait keeps your unit test surface area sane."

Hand-written mock:

```rust
#[derive(Clone)]
struct MockAuthorRepository {
    create_author_result: Arc<Mutex<Result<Author, CreateAuthorError>>>,
}

impl AuthorRepository for MockAuthorRepository {
    async fn create_author(&self, _: &CreateAuthorRequest) -> Result<Author, CreateAuthorError> {
        let mut guard = self.create_author_result.lock().await;
        let mut result = Err(CreateAuthorError::Unknown(anyhow!("substitute error")));
        mem::swap(guard.deref_mut(), &mut result);
        result
    }
}
```

The `Arc<Mutex<..>>` exists because the port requires `Clone`, but `CreateAuthorError` is not `Clone` (`anyhow::Error` is not `Clone`). `Arc` makes it cloneable; `Mutex` makes it `Sync`. The [`mockall`](https://docs.rs/mockall) crate (`#[automock]`) removes most of this boilerplate and adds argument matching and call counts.

## `main` is for bootstrapping

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    tracing_subscriber::fmt::init();

    let sqlite = Sqlite::new(&config.database_url).await?;
    let metrics = Prometheus::new();
    let email_client = EmailClient::new();
    let author_service = Service::new(sqlite, metrics, email_client);

    let server_config = HttpServerConfig { port: &config.server_port };
    let http_server = HttpServer::new(author_service, server_config).await?;
    http_server.run().await
}
```

- `main` chooses which adapters fill which ports, builds the Services, injects them into inbound adapters, and starts the program. Nothing else.
- Every import in `main` is the application's own type. `main` does not know about axum: `HttpServer` wraps it and owns routes, middleware, ports, and timeouts.
- `main` is hard to test. Less code in `main` means a smaller untested zone.
- Tests and `main` can both start the real server through `HttpServer`, so routing and middleware setup are not duplicated.

> "The less code you put in main, the smaller your testing dead zone."

## Why hexagons?

The shape is not significant. Alistair Cockburn drew a hexagon to show adapters surrounding the domain at the core, and the symmetry of inbound and outbound sides. Any polygon will do. The guide's mental model: the outside world changes constantly; the domain changes only when business requirements change; adapters are the bouncers that enforce the domain's dress code.

Other writing uses "use case" for one domain action (e.g. "create author"). It maps one-to-one to a Service method.

## Choosing domain boundaries

Two rules of thumb:

1. **A domain represents a tangible arm of the business.** A small blog is one "blog" domain. A platform like Medium has several: blogging, identity, billing, support. Domains talk to each other through their Service APIs.
2. **A domain includes all entities that must change together in one atomic operation.** "Atomic" here means the domain requires the changes to appear simultaneously, regardless of how storage achieves it.

Consequences:

- Cross-domain operations are never atomic. They are synchronous calls (`PostService::delete_by_author_id`) or asynchronous events (`AuthorDeletionEvent` on a queue).
- **If you leak transactions into business logic to make cross-domain operations atomic, your boundaries are wrong.** Merge those entities into one domain.
- An entity is not a database record. An `Author` aggregate may span many tables. Different domains may read the same data source and model it differently (a billing `Customer` does not care about authorship).
- **Start with a single, large domain.** Early guesses at boundaries are usually wrong. Small domains force you to write inter-domain glue before you know you need it and give up atomicity you may need later. Split a fat domain when real friction appears.

### Users and authentication

The `User` entity owns almost everything (profiles, settings, subscriptions). Strict atomicity puts everything in one domain.

- Small to medium apps with home-grown auth: one domain is fine.
- Large apps, or apps using a third-party provider such as Auth0: put `User` and auth in their own domain. Other domains keep a reference to the owner. Deletion propagates synchronously (`Service::delete_by_user_id`) or through events.
- At a certain scale, user deletion cannot be atomic in any architecture (Facebook takes up to 90 days). Hexagonal architecture only makes this explicit.

## Project layout

The guide's [example repository](https://github.com/howtocodeit/hexarch) (branch `3-simple-service`) splits `src/lib` into:

- `domain/` — business logic: models, ports, Service implementations (e.g. `domain::author::service::Service`);
- `inbound/` — inbound adapters (e.g. `inbound::http::HttpServer`);
- `outbound/` — outbound adapters (e.g. `outbound::sqlite::Sqlite`, `outbound::prometheus::Prometheus`, `outbound::email_client::EmailClient`).

Names and granularity are not sacred. Whatever convention you choose, document it for the team.

## Trade-offs

Strengths:

- Highly decoupled, so it is easy to evolve and scale.
- Much larger unit test surface, suited to code with complex failure modes.
- One source of truth per business domain and a simple dependency graph.
- A predictable home for every piece of code.

Costs:

- More code than the naive version: a wrapper around the HTTP framework, transport-to-domain conversion, an injected Service, a repository trait, and storage-to-domain conversion. This is not filler, but below some threshold it costs more than it saves.
- Conversions between transport and domain types cost CPU and memory. Negligible for most apps; unacceptable for embedded or ultra-low-latency systems.
- A learning barrier. It needs a clear technical vision and a team that understands the rationale. Adoption is a cultural investment, not only a technical one.

### When to use it

| Situation | Recommendation |
| --- | --- |
| Solo developer, personal project you can hold in your head | Skip it in production; it slows you down. Use small personal projects as a low-stakes playground to learn it. |
| Little business logic (thin CRUD) | Skip ports and adapters; there is nothing to encapsulate. Use a repository only if you want to test handlers without the database; otherwise integration test. |
| Startup that intends to scale | Use it from day one. Start as a coarse-grained hexagonal monolith, not microservices. Clear layers also make AI-generated unit tests more accurate. |
| Big, tangled monolith | Refactor to a hexagonal monolith before extracting microservices. Pull one domain behind ports inside the monolith, let its API stabilise, then extract it. Prioritise the code with the biggest organisational and resource pressure. |
| Greenfield project in an established company | Use it. The logic is non-trivial, dependencies will change, and many people will maintain the code for years. Get team buy-in. |
| High-performance (zero-copy, HFT, embedded) | Skip it. You cannot afford the conversions. |

> "If you can't build a modular monolith, you're not qualified to run microservices."

A hexagonal domain does not care where requests come from: another in-process domain, an internal RPC, or a public REST call. That is why extracting a stable domain into a microservice later is a relatively simple step.

## Exercises from the guide

1. List the hard-coded dependencies of a large codebase you work on: the ones whose disappearance would force changes in unrelated code.
2. Find where those dependencies do not belong but would be hard to remove: handlers talking to databases, middleware configured in `main`, Kafka errors outside Kafka code, RPC models outside RPC modules.
3. Add async `find` and `find_all` methods to `AuthorRepository`, with return types that cover every error scenario.
4. Implement the extended `AuthorRepository` for a database of your choice. Decide (and be ready to defend) whether unexpected errors panic or return a catch-all variant.
