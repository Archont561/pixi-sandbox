Feature: Transport integrity
  `doctor --verify` judges a packed transport against its manifest before anyone trusts it:
  a healthy transport earns the full verdict, and a single wrong byte loses it. The verdict
  is read-only; it never writes.

  Scenario: A packed transport round-trips through verification
    Given a packed transport
    When the operator verifies the transport
    Then every declared byte is reported to match
    And the verdict is honest about the embedded bootstrap probe

  Scenario: A tampered byte loses the healthy verdict
    Given a packed transport carrying one tampered byte
    When the operator verifies the transport
    Then the verification is refused
    And no healthy verdict is reported
    And the refusal names the tampered blob

  Scenario: A tampered shard part is named in the refusal
    Given a packed transport carrying a tampered shard part
    When the operator verifies the transport
    Then the verification is refused
    And the refusal names the tampered part
