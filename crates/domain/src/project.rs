use std::collections::HashSet;

use thiserror::Error;

use crate::id::Id;
use crate::link::Link;
use crate::value_objects::{ProjectDescription, ProjectName, ProjectStatus, TechnologyName};

/// Identifies a [`Project`].
pub type ProjectId = Id<Project>;

/// A Northbyte Studios project, as described in `docs/PROJECT_SPEC.md`
/// section 4 ("Information Model" -> "Projects").
///
/// `Releases`, `Public roadmap information` and `Updates` are named in the
/// specification but given no field-level structure, so they are not
/// modeled here; see the Milestone 02 report for that open question.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    id: ProjectId,
    name: ProjectName,
    description: ProjectDescription,
    status: ProjectStatus,
    repository: Option<Link>,
    documentation: Option<Link>,
    technologies: Vec<TechnologyName>,
}

impl Project {
    pub fn new(
        id: ProjectId,
        name: ProjectName,
        description: ProjectDescription,
        status: ProjectStatus,
        repository: Option<Link>,
        documentation: Option<Link>,
        technologies: Vec<TechnologyName>,
    ) -> Result<Self, ProjectError> {
        let mut seen = HashSet::with_capacity(technologies.len());
        for technology in &technologies {
            let key = technology.as_str().to_lowercase();
            if !seen.insert(key) {
                return Err(ProjectError::DuplicateTechnology {
                    technology: technology.as_str().to_string(),
                });
            }
        }

        Ok(Self { id, name, description, status, repository, documentation, technologies })
    }

    pub fn id(&self) -> ProjectId {
        self.id
    }

    pub fn name(&self) -> &ProjectName {
        &self.name
    }

    pub fn description(&self) -> &ProjectDescription {
        &self.description
    }

    pub fn status(&self) -> &ProjectStatus {
        &self.status
    }

    pub fn repository(&self) -> Option<&Link> {
        self.repository.as_ref()
    }

    pub fn documentation(&self) -> Option<&Link> {
        self.documentation.as_ref()
    }

    pub fn technologies(&self) -> &[TechnologyName] {
        &self.technologies
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ProjectError {
    #[error("technology '{technology}' is listed more than once")]
    DuplicateTechnology { technology: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_project() -> Project {
        Project::new(
            ProjectId::new(),
            ProjectName::parse("P4inz").unwrap(),
            ProjectDescription::parse("Northbyte Studios' community intelligence bot").unwrap(),
            ProjectStatus::parse("active").unwrap(),
            Some(Link::parse("https://github.com/p4inz-code/p4inz").unwrap()),
            None,
            vec![
                TechnologyName::parse("Rust").unwrap(),
                TechnologyName::parse("PostgreSQL").unwrap(),
            ],
        )
        .unwrap()
    }

    #[test]
    fn constructs_with_valid_fields() {
        let project = valid_project();
        assert_eq!(project.name().as_str(), "P4inz");
        assert_eq!(project.description().as_str(), "Northbyte Studios' community intelligence bot");
        assert_eq!(project.status().as_str(), "active");
        assert_eq!(project.technologies().len(), 2);
        assert!(project.repository().is_some());
        assert!(project.documentation().is_none());
    }

    #[test]
    fn documentation_link_is_returned_when_present() {
        let project = Project::new(
            ProjectId::new(),
            ProjectName::parse("P4inz").unwrap(),
            ProjectDescription::parse("desc").unwrap(),
            ProjectStatus::parse("active").unwrap(),
            None,
            Some(Link::parse("https://docs.p4inz.dev").unwrap()),
            Vec::new(),
        )
        .unwrap();

        assert_eq!(project.documentation().map(Link::as_str), Some("https://docs.p4inz.dev"));
    }

    #[test]
    fn allows_no_repository_or_documentation() {
        let project = Project::new(
            ProjectId::new(),
            ProjectName::parse("P4inz").unwrap(),
            ProjectDescription::parse("A project without links yet").unwrap(),
            ProjectStatus::parse("planned").unwrap(),
            None,
            None,
            Vec::new(),
        )
        .unwrap();

        assert!(project.repository().is_none());
        assert!(project.documentation().is_none());
        assert!(project.technologies().is_empty());
    }

    #[test]
    fn rejects_duplicate_technology_case_insensitively() {
        let result = Project::new(
            ProjectId::new(),
            ProjectName::parse("P4inz").unwrap(),
            ProjectDescription::parse("desc").unwrap(),
            ProjectStatus::parse("active").unwrap(),
            None,
            None,
            vec![TechnologyName::parse("Rust").unwrap(), TechnologyName::parse("rust").unwrap()],
        );

        assert_eq!(
            result,
            Err(ProjectError::DuplicateTechnology { technology: "rust".to_string() })
        );
    }

    #[test]
    fn distinct_projects_have_distinct_ids() {
        let a = valid_project();
        let b = valid_project();
        assert_ne!(a.id(), b.id());
    }
}
