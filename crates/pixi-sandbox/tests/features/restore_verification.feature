Feature: Restore verification
  A restore verifies every declared byte of the transport before it writes anything, and a
  verified restore leaves a complete project behind. Nothing reaches the project on a failed
  verdict, and the failure report is the evidence an airlock operator acts on.

  Scenario: A verified restore writes the project and cleans its scratch
    Given a packed transport and an empty project
    When the operator restores the transport into the project
    Then the restore reports completion
    And the demo environment is materialised in the project
    And no restore scratch is left behind

  Scenario: A tampered transport is refused with nothing written
    Given a packed transport with one tampered byte
    When the operator restores the transport into the project
    Then the restore is refused
    And the refusal reports that nothing was written
    And the refusal names the tampered blob
    And the project receives no environment

  Scenario: A missing transport is refused before writing
    Given a branch location that does not exist
    When the operator restores the transport into the project
    Then the restore is refused
    And the refusal names the branch location
    And the project receives no environment

  Scenario: A verify-only run reports the verdict and writes nothing
    Given a packed transport and an empty project
    When the operator asks restore to verify only
    Then the verdict reports that no project files were written
    And the project receives no environment
