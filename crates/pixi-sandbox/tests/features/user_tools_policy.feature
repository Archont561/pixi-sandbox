Feature: User-tools policy
  A verified restore decides how the restored tools reach the operator. Registration of the
  pixi and pixi-sandbox launchers in the operator's home is the default, `skip` opts out for
  CI, the policy can travel as an environment variable, and a bootstrap packed before the
  policy existed reports honestly — never announcing a registration that did not happen.

  Scenario: A verified restore registers the tools by default
    Given a packed transport and an isolated home
    When the operator restores the transport without stating a policy
    Then the restore registers pixi and pixi-sandbox in the home
    And the launchers exec the manifest-verified tool copies
    And the shell profile gains exactly one managed PATH block
    And the report says an already-running shell cannot be changed

  Scenario: The skip policy touches nothing outside the project
    Given a packed transport and an isolated home
    When the operator restores the transport asking to skip registration
    Then the report says nothing was registered
    And nothing is registered in the home
    And the project still receives its environment

  Scenario: The policy can travel as an environment variable
    Given a packed transport and an isolated home
    And the environment selects the skip policy
    When the operator restores the transport without stating a policy
    Then nothing is registered in the home

  Scenario: A pre-policy bootstrap reports honestly instead of announcing
    Given a bootstrap packed before user-tools
    When the operator runs the restore script asking to register
    Then the report names the bundled version that ignored the policy
    And the report does not claim a registration
    And nothing is registered in the home
