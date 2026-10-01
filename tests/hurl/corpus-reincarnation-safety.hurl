# The `reincarnation-safety` family of the uVRR compliance corpus, replayed over the
# transport (`docs/uvrr-host-compliance.md` §8): 1 cases, each POSTed
# to the host's case endpoint with its expectation in the body, so the host
# compares its own capture against the committed bytes.
#
# Generated from `tests/compliance/corpus/reincarnation-safety.json` by
# `cargo test --features conformance_host --test conformance_host -- --ignored
# export_hurl`. The gate `the_committed_hurl_suite_equals_the_generated_suite`
# rebuilds this file in memory and compares: a corpus change that was not
# re-exported fails the gate. Edit the corpus, then re-export.

# reincarnation-safety-the-forced-sequence-seats-the-bumped-life
# A crashed member MUST return through a bumped identity and the forced sequence: the old identity is evicted, the bumped life is seated at weight one, every journal agrees, the witness lists drop the promoted node, and the deferred marker round lands at the seated witness
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "reincarnation-safety-the-forced-sequence-seats-the-bumped-life",
  "family": "reincarnation-safety",
  "theorem": "ReincarnationGeneral.forced_sequence_era_safe",
  "clause": "A crashed member MUST return through a bumped identity and the forced sequence: the old identity is evicted, the bumped life is seated at weight one, every journal agrees, the witness lists drop the promoted node, and the deferred marker round lands at the seated witness",
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
      "op": "propose",
      "node": "1:1",
      "payload": "missed"
    },
    {
      "op": "deliver_all"
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
      "op": "announce",
      "node": "3:2",
      "old": "3:1"
    },
    {
      "op": "deliver_all"
    },
    {
      "op": "tick",
      "node": "1:1"
    },
    {
      "op": "tick",
      "node": "1:1"
    },
    {
      "op": "tick",
      "node": "1:1"
    },
    {
      "op": "tick",
      "node": "1:1"
    },
    {
      "op": "deliver_all"
    }
  ],
  "input": {
    "op": "announce",
    "node": "3:2",
    "old": "3:1"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:2",
        "to": "1:1",
        "wire": "0000000d000000020000000100000000000000000d000300010003000200000000000000040000000000000004"
      },
      {
        "from": "3:2",
        "to": "2:1",
        "wire": "0000000d000000020000000100000000000000000d000300010003000200000000000000040000000000000004"
      },
      {
        "from": "3:2",
        "to": "3:1",
        "wire": "0000000d000000020000000100000000000000000d000300010003000200000000000000040000000000000004"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000020000000200000001000000000000000502000000000000000500000002020900000002030003000208000300010000000000000004"
      },
      {
        "from": "2:1",
        "to": "3:2",
        "wire": "000000020000000200000001000000000000000502000000000000000500000002020900000002030003000208000300010000000000000004"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000020000000200000001000000000000000502000000000000000500000002020900000002030003000208000300010000000000000004"
      },
      {
        "from": "2:1",
        "to": "3:2",
        "wire": "000000020000000200000001000000000000000502000000000000000500000002020900000002030003000208000300010000000000000004"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000030000000200000001000000000000000503"
      },
      {
        "from": "3:2",
        "to": "2:1",
        "wire": "000000030000000200000001000000000000000503"
      },
      {
        "from": "3:2",
        "to": "2:1",
        "wire": "000000030000000200000001000000000000000503"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "0000000400000002000000010000000000000005040000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:2",
        "wire": "0000000400000002000000010000000000000005040000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000400000002000000010000000000000005040000000000000005"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 2,
        "view": 1,
        "accepted": 5,
        "committed": 5,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "missed",
          "batch [decrement 3:1, join 3:2 at 2]",
          "batch [increment 3:2, leave 3:1]"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:2"
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
        "era": 2,
        "view": 1,
        "accepted": 5,
        "committed": 5,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "missed",
          "batch [decrement 3:1, join 3:2 at 2]",
          "batch [increment 3:2, leave 3:1]"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:2"
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
        "status": "Normal",
        "era": 2,
        "view": 1,
        "accepted": 5,
        "committed": 5,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "missed",
          "batch [decrement 3:1, join 3:2 at 2]",
          "batch [increment 3:2, leave 3:1]"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:2"
        ],
        "weights": [
          1,
          1,
          1
        ],
        "markers": [
          "Joining@3:2"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "reincarnation-safety-the-forced-sequence-seats-the-bumped-life"
jsonpath "$.family" == "reincarnation-safety"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 13
