# The `casting-vote` family of the uVRR compliance corpus, replayed over the
# transport (`docs/uvrr-host-compliance.md` §8): 2 cases, each POSTed
# to the host's case endpoint with its expectation in the body, so the host
# compares its own capture against the committed bytes.
#
# Generated from `tests/compliance/corpus/casting-vote.json` by
# `cargo test --features conformance_host --test conformance_host -- --ignored
# export_hurl`. The gate `the_committed_hurl_suite_equals_the_generated_suite`
# rebuilds this file in memory and compares: a corpus change that was not
# re-exported fails the gate. Edit the corpus, then re-export.

# casting-vote-phase-one-completes-through-the-heavy-member
# A member whose weight seats it inside every majority MUST be the member through which phase one completes: the commit carries its acceptance
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "casting-vote-phase-one-completes-through-the-heavy-member",
  "family": "casting-vote",
  "theorem": "CastingVote.completes_phase1",
  "clause": "A member whose weight seats it inside every majority MUST be the member through which phase one completes: the commit carries its acceptance",
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
      "op": "reconfigure",
      "node": "1:1",
      "system": {
        "kind": "increment",
        "node": "3:1"
      }
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
    "node": "1:1",
    "payload": "through-the-heavy-member"
  },
  "expect": {
    "deliveries": [
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "0000000200000001000000000000000000000004020000000000000004000000020100000000766563740000000000000001000000187468726f7567682d7468652d68656176792d6d656d6265720000000000000003"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000200000001000000000000000000000004020000000000000004000000020100000000766563740000000000000001000000187468726f7567682d7468652d68656176792d6d656d6265720000000000000003"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000030000000100000000000000000000000403"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000030000000100000000000000000000000403"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "0000000400000001000000000000000000000004040000000000000004"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000400000001000000000000000000000004040000000000000004"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 0,
        "accepted": 4,
        "committed": 4,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "increment 3:1",
          "through-the-heavy-member"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1"
        ],
        "weights": [
          1,
          1,
          2
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
        "committed": 4,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "increment 3:1",
          "through-the-heavy-member"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1"
        ],
        "weights": [
          1,
          1,
          2
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
        "accepted": 4,
        "committed": 4,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "increment 3:1",
          "through-the-heavy-member"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1"
        ],
        "weights": [
          1,
          1,
          2
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
jsonpath "$.id" == "casting-vote-phase-one-completes-through-the-heavy-member"
jsonpath "$.family" == "casting-vote"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 6

# casting-vote-without-the-heavy-member-no-quorum-completes
# Without the member whose weight seats it inside every majority, no phase-one quorum MUST form: nothing commits and no frontier of commitment moves
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "casting-vote-without-the-heavy-member-no-quorum-completes",
  "family": "casting-vote",
  "theorem": "CastingVote.guard_preserved",
  "clause": "Without the member whose weight seats it inside every majority, no phase-one quorum MUST form: nothing commits and no frontier of commitment moves",
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
      "op": "reconfigure",
      "node": "1:1",
      "system": {
        "kind": "increment",
        "node": "3:1"
      }
    },
    {
      "op": "deliver_all"
    },
    {
      "op": "settle"
    },
    {
      "op": "crash",
      "node": "3:1"
    }
  ],
  "input": {
    "op": "propose",
    "node": "1:1",
    "payload": "without-the-heavy-member"
  },
  "expect": {
    "deliveries": [
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000020000000100000000000000000000000402000000000000000400000002010000000076656374000000000000000100000018776974686f75742d7468652d68656176792d6d656d6265720000000000000003"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000020000000100000000000000000000000402000000000000000400000002010000000076656374000000000000000100000018776974686f75742d7468652d68656176792d6d656d6265720000000000000003"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000030000000100000000000000000000000403"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 0,
        "accepted": 4,
        "committed": 3,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "increment 3:1",
          "without-the-heavy-member"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1"
        ],
        "weights": [
          1,
          1,
          2
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
        "committed": 3,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "increment 3:1",
          "without-the-heavy-member"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1"
        ],
        "weights": [
          1,
          1,
          2
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
jsonpath "$.id" == "casting-vote-without-the-heavy-member-no-quorum-completes"
jsonpath "$.family" == "casting-vote"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 3
