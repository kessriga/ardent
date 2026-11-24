# Spec Delta

## ADDED Requirements

### Requirement: Dashboard settings

In projects with the view host, Ardent SHALL read each enabled view plugin's dashboard settings from
`views.<plugin>.dashboards.<id>` in `.ardent/config.yaml`. Ardent SHALL pass only that dashboard's named fields to its
`render` call. Each value SHALL be text, a number, a boolean or a list of texts. The plugin SHALL validate its settings
and apply its defaults when fields are absent.

#### Scenario: Limit recent comments

- **WHEN** the project configures `views: {comments: {dashboards: {recent: {limit: 20}}}}` and opens recent comments
- **THEN** the host passes the numeric field `limit` with value `20` to the comments plugin's `recent` dashboard

#### Scenario: Dashboard uses its defaults

- **WHEN** the project enables the comments plugin with `views: {comments: {}}` and opens recent comments
- **THEN** the host passes an empty set of settings and the plugin applies its defaults
