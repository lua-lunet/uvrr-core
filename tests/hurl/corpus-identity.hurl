# The `identity` family of the uVRR compliance corpus, replayed over the
# transport (`docs/uvrr-host-compliance.md` §8): 2 cases, each POSTed
# to the host's case endpoint with its expectation in the body, so the host
# compares its own capture against the committed bytes.
#
# Generated from `tests/compliance/corpus/identity.json` by
# `cargo test --features conformance_host --test conformance_host -- --ignored
# export_hurl`. The gate `the_committed_hurl_suite_equals_the_generated_suite`
# rebuilds this file in memory and compares: a corpus change that was not
# re-exported fails the gate. Edit the corpus, then re-export.

# identity-the-announcement-packs-the-durable-pair
# The reincarnation announcement MUST pack the durable pair on the wire, the old identity and its bump, and a hearing backup MUST drop it by name and list the announcer
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "identity-the-announcement-packs-the-durable-pair",
  "family": "identity",
  "theorem": "IdentityLaw.pack_wire",
  "clause": "The reincarnation announcement MUST pack the durable pair on the wire, the old identity and its bump, and a hearing backup MUST drop it by name and list the announcer",
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
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "crashed"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "2:1",
    "from": "3:2",
    "wire": "0000000d000000010000000000000000000000000d000300010003000200000000000000020000000000000002"
  },
  "expect": {
    "deliveries": [],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 0,
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
          "Joining@1:1"
        ],
        "witnesses": []
      },
      {
        "node": "2:1",
        "status": "Normal",
        "era": 1,
        "view": 0,
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
        "witnesses": [
          "3:2"
        ]
      },
      {
        "node": "3:1",
        "status": null,
        "era": null,
        "view": null,
        "accepted": null,
        "committed": null,
        "applied": null,
        "journal": [],
        "members": null,
        "weights": null,
        "markers": [],
        "witnesses": []
      },
      {
        "node": "3:2",
        "status": "Restarting",
        "era": 1,
        "view": 0,
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
        "markers": [],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "identity-the-announcement-packs-the-durable-pair"
jsonpath "$.family" == "identity"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# identity-each-life-is-one-counter-past-and-never-revisited
# Each bump MUST advance the crash counter exactly one past the last identity the disk saw: a value is burnt once and serves one life, and nothing self-resets to zero
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "identity-each-life-is-one-counter-past-and-never-revisited",
  "family": "identity",
  "theorem": "IdentityLaw.lawful_life_separates",
  "clause": "Each bump MUST advance the crash counter exactly one past the last identity the disk saw: a value is burnt once and serves one life, and nothing self-resets to zero",
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
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "crashed"
    },
    {
      "op": "crash",
      "node": "3:2"
    }
  ],
  "input": {
    "op": "restart",
    "node": "3:2",
    "kind": "crashed"
  },
  "expect": {
    "deliveries": [],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 0,
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
          "Joining@1:1"
        ],
        "witnesses": []
      },
      {
        "node": "2:1",
        "status": "Normal",
        "era": 1,
        "view": 0,
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
        "status": null,
        "era": null,
        "view": null,
        "accepted": null,
        "committed": null,
        "applied": null,
        "journal": [],
        "members": null,
        "weights": null,
        "markers": [],
        "witnesses": []
      },
      {
        "node": "3:2",
        "status": null,
        "era": null,
        "view": null,
        "accepted": null,
        "committed": null,
        "applied": null,
        "journal": [],
        "members": null,
        "weights": null,
        "markers": [],
        "witnesses": []
      },
      {
        "node": "3:3",
        "status": "Restarting",
        "era": 1,
        "view": 0,
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
        "markers": [],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "identity-each-life-is-one-counter-past-and-never-revisited"
jsonpath "$.family" == "identity"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0
