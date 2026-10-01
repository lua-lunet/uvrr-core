# The `view-selection` family of the uVRR compliance corpus, replayed over the
# transport (`docs/uvrr-host-compliance.md` §8): 2 cases, each POSTed
# to the host's case endpoint with its expectation in the body, so the host
# compares its own capture against the committed bytes.
#
# Generated from `tests/compliance/corpus/view-selection.json` by
# `cargo test --features conformance_host --test conformance_host -- --ignored
# export_hurl`. The gate `the_committed_hurl_suite_equals_the_generated_suite`
# rebuilds this file in memory and compares: a corpus change that was not
# re-exported fails the gate. Edit the corpus, then re-export.

# view-selection-the-failover-collects-elects-and-installs
# The survivors of a silent primary MUST complete the failover: the surviving evidence reaches the new primary, the highest-ranked report is selected, and the installed view is announced to every survivor
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "view-selection-the-failover-collects-elects-and-installs",
  "family": "view-selection",
  "theorem": "ViewSelection.newer_view_wins",
  "clause": "The survivors of a silent primary MUST complete the failover: the surviving evidence reaches the new primary, the highest-ranked report is selected, and the installed view is announced to every survivor",
  "keywords": [
    "MUST"
  ],
  "setup": [
    {
      "op": "provision",
      "nodes": 3,
      "timeout": 3
    },
    {
      "op": "settle"
    },
    {
      "op": "crash",
      "node": "1:1"
    },
    {
      "op": "tick",
      "node": "2:1"
    },
    {
      "op": "tick",
      "node": "2:1"
    },
    {
      "op": "tick",
      "node": "2:1"
    },
    {
      "op": "tick",
      "node": "2:1"
    }
  ],
  "input": {
    "op": "tick",
    "node": "3:1"
  },
  "expect": {
    "deliveries": [
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000050000000100000001000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000050000000100000001000000000000000005"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000050000000100000001000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000050000000100000001000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000600000001000000010000000000000002060000000100000000000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "0000000700000001000000010000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000700000001000000010000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": null,
        "era": null,
        "view": null,
        "accepted": null,
        "committed": null,
        "applied": null,
        "journal": [],
        "members": null,
        "weights": null,
        "markers": [
          "Joining@1:1"
        ],
        "witnesses": []
      },
      {
        "node": "2:1",
        "status": "Normal",
        "era": 1,
        "view": 1,
        "accepted": 2,
        "committed": 2,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1"
        ],
        "weights": [
          1,
          1,
          1
        ],
        "markers": [
          "Joining@2:1"
        ],
        "witnesses": []
      },
      {
        "node": "3:1",
        "status": "Normal",
        "era": 1,
        "view": 1,
        "accepted": 2,
        "committed": 2,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1"
        ],
        "weights": [
          1,
          1,
          1
        ],
        "markers": [
          "Joining@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "view-selection-the-failover-collects-elects-and-installs"
jsonpath "$.family" == "view-selection"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 7

# view-selection-the-committed-prefix-survives-and-serves
# The selected history MUST preserve the committed prefix, and the cluster MUST serve under the installed view: a proposal commits and every survivor journals the identical history
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "view-selection-the-committed-prefix-survives-and-serves",
  "family": "view-selection",
  "theorem": "ViewSelection.quorum_preserves",
  "clause": "The selected history MUST preserve the committed prefix, and the cluster MUST serve under the installed view: a proposal commits and every survivor journals the identical history",
  "keywords": [
    "MUST"
  ],
  "setup": [
    {
      "op": "provision",
      "nodes": 3,
      "timeout": 3
    },
    {
      "op": "settle"
    },
    {
      "op": "crash",
      "node": "1:1"
    },
    {
      "op": "tick",
      "node": "2:1"
    },
    {
      "op": "tick",
      "node": "2:1"
    },
    {
      "op": "tick",
      "node": "2:1"
    },
    {
      "op": "tick",
      "node": "2:1"
    },
    {
      "op": "tick",
      "node": "3:1"
    },
    {
      "op": "deliver_all"
    },
    {
      "op": "settle"
    }
  ],
  "input": {
    "op": "propose",
    "node": "2:1",
    "payload": "re-elected"
  },
  "expect": {
    "deliveries": [
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "00000002000000010000000100000000000000030200000000000000030000000101000000007665637400000000000000010000000a72652d656c65637465640000000000000002"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "00000002000000010000000100000000000000030200000000000000030000000101000000007665637400000000000000010000000a72652d656c65637465640000000000000002"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000303"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "0000000400000001000000010000000000000003040000000000000003"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000400000001000000010000000000000003040000000000000003"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": null,
        "era": null,
        "view": null,
        "accepted": null,
        "committed": null,
        "applied": null,
        "journal": [],
        "members": null,
        "weights": null,
        "markers": [
          "Joining@1:1"
        ],
        "witnesses": []
      },
      {
        "node": "2:1",
        "status": "Normal",
        "era": 1,
        "view": 1,
        "accepted": 3,
        "committed": 3,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "re-elected"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1"
        ],
        "weights": [
          1,
          1,
          1
        ],
        "markers": [
          "Joining@2:1"
        ],
        "witnesses": []
      },
      {
        "node": "3:1",
        "status": "Normal",
        "era": 1,
        "view": 1,
        "accepted": 3,
        "committed": 3,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "re-elected"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1"
        ],
        "weights": [
          1,
          1,
          1
        ],
        "markers": [
          "Joining@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "view-selection-the-committed-prefix-survives-and-serves"
jsonpath "$.family" == "view-selection"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 5
