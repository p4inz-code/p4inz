//! P4inz application subsystem.
//!
//! Use cases that orchestrate domain/knowledge/AI logic and the
//! persistence/handler contracts (ports) infrastructure and adapters
//! implement, applying authorization/audit before results are returned
//! (`docs/development/implementation_plan.md` section 12). This crate
//! depends on `p4inz-ai`'s provider *abstraction* only — never a concrete
//! provider, transport, or other infrastructure implementation
//! (`docs/architecture/dependency-rules.md`).

mod ai_question_handler;
mod knowledge_search;
mod project_repository;
mod question;
mod question_handler;
mod register_project;
mod response_validation;

pub use ai_question_handler::AiQuestionHandler;
pub use knowledge_search::{KnowledgeSearch, SEARCH_PERMISSION, SearchKnowledge};
pub use project_repository::ProjectRepository;
pub use question::{QUESTION_MAX_LEN, Question, QuestionError};
pub use question_handler::{QuestionHandler, UnavailableQuestionHandler};
pub use register_project::{RegisterProject, RegisterProjectInput};
pub use response_validation::{ResponseValidationError, validate_response};
