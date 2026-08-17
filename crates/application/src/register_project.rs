use p4inz_domain::{
    Link, Project, ProjectDescription, ProjectId, ProjectName, ProjectStatus, TechnologyName,
};
use p4inz_errors::{AppResult, ErrorKind, IntoAppError};

use crate::project_repository::ProjectRepository;

/// Raw, unvalidated input for registering a new [`Project`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisterProjectInput {
    pub name: String,
    pub description: String,
    pub status: String,
    pub repository: Option<String>,
    pub documentation: Option<String>,
    pub technologies: Vec<String>,
}

/// Registers a new project: validates raw input into domain types, then
/// persists it through a [`ProjectRepository`].
///
/// Generic over the repository rather than `dyn ProjectRepository`: the
/// trait's methods are native `async fn`s, which are not object-safe. If a
/// later milestone needs runtime polymorphism across repository
/// implementations (e.g. wiring a concrete type at a composition root), it
/// should introduce `async-trait` (or manual boxed futures) then, once that
/// need is concrete.
pub struct RegisterProject<'a, R: ProjectRepository> {
    repository: &'a R,
}

impl<'a, R: ProjectRepository> RegisterProject<'a, R> {
    pub fn new(repository: &'a R) -> Self {
        Self { repository }
    }

    pub async fn execute(&self, input: RegisterProjectInput) -> AppResult<Project> {
        let name = ProjectName::parse(input.name)
            .into_app_error(ErrorKind::Validation, "invalid project name")?;
        let description = ProjectDescription::parse(input.description)
            .into_app_error(ErrorKind::Validation, "invalid project description")?;
        let status = ProjectStatus::parse(input.status)
            .into_app_error(ErrorKind::Validation, "invalid project status")?;
        let repository_link = input
            .repository
            .map(Link::parse)
            .transpose()
            .into_app_error(ErrorKind::Validation, "invalid repository link")?;
        let documentation_link = input
            .documentation
            .map(Link::parse)
            .transpose()
            .into_app_error(ErrorKind::Validation, "invalid documentation link")?;
        let technologies = input
            .technologies
            .into_iter()
            .map(TechnologyName::parse)
            .collect::<Result<Vec<_>, _>>()
            .into_app_error(ErrorKind::Validation, "invalid technology name")?;

        let project = Project::new(
            ProjectId::new(),
            name,
            description,
            status,
            repository_link,
            documentation_link,
            technologies,
        )
        .into_app_error(ErrorKind::Validation, "invalid project")?;

        self.repository.save(&project).await?;

        Ok(project)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use p4inz_errors::AppError;

    use super::*;

    #[derive(Default)]
    struct InMemoryProjectRepository {
        projects: Mutex<Vec<Project>>,
    }

    impl ProjectRepository for InMemoryProjectRepository {
        async fn save(&self, project: &Project) -> AppResult<()> {
            self.projects.lock().unwrap().push(project.clone());
            Ok(())
        }

        async fn find_by_id(&self, id: ProjectId) -> AppResult<Option<Project>> {
            Ok(self.projects.lock().unwrap().iter().find(|p| p.id() == id).cloned())
        }
    }

    struct FailingProjectRepository;

    impl ProjectRepository for FailingProjectRepository {
        async fn save(&self, _project: &Project) -> AppResult<()> {
            Err(AppError::unavailable("database is down"))
        }

        async fn find_by_id(&self, _id: ProjectId) -> AppResult<Option<Project>> {
            Err(AppError::unavailable("database is down"))
        }
    }

    fn valid_input() -> RegisterProjectInput {
        RegisterProjectInput {
            name: "P4inz".to_string(),
            description: "Northbyte Studios' community intelligence bot".to_string(),
            status: "active".to_string(),
            repository: Some("https://github.com/p4inz-code/p4inz".to_string()),
            documentation: None,
            technologies: vec!["Rust".to_string(), "PostgreSQL".to_string()],
        }
    }

    #[tokio::test]
    async fn registers_and_persists_a_valid_project() {
        let repository = InMemoryProjectRepository::default();
        let use_case = RegisterProject::new(&repository);

        let project = use_case.execute(valid_input()).await.unwrap();

        assert_eq!(project.name().as_str(), "P4inz");
        let found = repository.find_by_id(project.id()).await.unwrap();
        assert_eq!(found, Some(project));
    }

    #[tokio::test]
    async fn rejects_invalid_name_without_touching_repository() {
        let repository = InMemoryProjectRepository::default();
        let use_case = RegisterProject::new(&repository);

        let mut input = valid_input();
        input.name = "   ".to_string();

        let err = use_case.execute(input).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
        assert!(repository.projects.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_invalid_description_without_touching_repository() {
        let repository = InMemoryProjectRepository::default();
        let use_case = RegisterProject::new(&repository);

        let mut input = valid_input();
        input.description = "   ".to_string();

        let err = use_case.execute(input).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
        assert!(repository.projects.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_invalid_status_without_touching_repository() {
        let repository = InMemoryProjectRepository::default();
        let use_case = RegisterProject::new(&repository);

        let mut input = valid_input();
        input.status = "   ".to_string();

        let err = use_case.execute(input).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
        assert!(repository.projects.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_an_invalid_repository_link() {
        let repository = InMemoryProjectRepository::default();
        let use_case = RegisterProject::new(&repository);

        let mut input = valid_input();
        input.repository = Some("not a valid url".to_string());

        let err = use_case.execute(input).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
        assert!(repository.projects.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejects_an_invalid_documentation_link() {
        let repository = InMemoryProjectRepository::default();
        let use_case = RegisterProject::new(&repository);

        let mut input = valid_input();
        input.documentation = Some("not a valid url".to_string());

        let err = use_case.execute(input).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
        assert!(repository.projects.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn accepts_a_valid_documentation_link() {
        let repository = InMemoryProjectRepository::default();
        let use_case = RegisterProject::new(&repository);

        let mut input = valid_input();
        input.documentation = Some("https://docs.p4inz.dev".to_string());

        let project = use_case.execute(input).await.unwrap();
        assert!(project.documentation().is_some());
    }

    #[tokio::test]
    async fn rejects_duplicate_technologies() {
        let repository = InMemoryProjectRepository::default();
        let use_case = RegisterProject::new(&repository);

        let mut input = valid_input();
        input.technologies = vec!["Rust".to_string(), "rust".to_string()];

        let err = use_case.execute(input).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Validation);
    }

    #[tokio::test]
    async fn propagates_repository_failure() {
        let repository = FailingProjectRepository;
        let use_case = RegisterProject::new(&repository);

        let err = use_case.execute(valid_input()).await.unwrap_err();
        assert_eq!(err.kind(), ErrorKind::Unavailable);
    }
}
