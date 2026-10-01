# The `fuse` family of the uVRR compliance corpus, replayed over the
# transport (`docs/uvrr-host-compliance.md` §8): 2 cases, each POSTed
# to the host's case endpoint with its expectation in the body, so the host
# compares its own capture against the committed bytes.
#
# Generated from `tests/compliance/corpus/fuse.json` by
# `cargo test --features conformance_host --test conformance_host -- --ignored
# export_hurl`. The gate `the_committed_hurl_suite_equals_the_generated_suite`
# rebuilds this file in memory and compares: a corpus change that was not
# re-exported fails the gate. Edit the corpus, then re-export.

# fuse-the-valid-batch-is-accepted-as-a-whole
# A Fuse datagram MUST be accepted as a whole: every packed operation journals at its own slot in batch order, the frontiers advance per packed operation, and the accept answers once
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "fuse-the-valid-batch-is-accepted-as-a-whole",
  "family": "fuse",
  "theorem": "Fuse.decide_all",
  "clause": "A Fuse datagram MUST be accepted as a whole: every packed operation journals at its own slot in batch order, the frontiers advance per packed operation, and the accept answers once",
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
    }
  ],
  "input": {
    "op": "deliver",
    "to": "2:1",
    "from": "1:1",
    "wire": "0000000e000000010000000000000000000000030e000000020700040001000000030300040001"
  },
  "expect": {
    "deliveries": [
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "0000000f000000010000000000000000000000040f0000000200000000000000030000000000000004"
      }
    ],
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
        "accepted": 4,
        "committed": 2,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "join 4:1 at 3",
          "increment 4:1"
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
          "Joining@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "fuse-the-valid-batch-is-accepted-as-a-whole"
jsonpath "$.family" == "fuse"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# fuse-the-stale-ballot-is-refused-as-a-whole
# A Fuse at a ballot the receiver has passed MUST be refused as a whole: nothing journals, nothing installs, and no partial accept is visible
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "fuse-the-stale-ballot-is-refused-as-a-whole",
  "family": "fuse",
  "theorem": "Fuse.same_ballot_era_guard",
  "clause": "A Fuse at a ballot the receiver has passed MUST be refused as a whole: nothing journals, nothing installs, and no partial accept is visible",
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
      "op": "deliver_all"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "2:1",
    "from": "1:1",
    "wire": "0000000e000000010000000000000000000000030e000000020700040001000000030300040001"
  },
  "expect": {
    "deliveries": [],
    "post": [
      {
        "node": "1:1",
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
jsonpath "$.id" == "fuse-the-stale-ballot-is-refused-as-a-whole"
jsonpath "$.family" == "fuse"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0
