# The `reconfiguration` family of the uVRR compliance corpus, replayed over the
# transport (`docs/uvrr-host-compliance.md` §8): 4 cases, each POSTed
# to the host's case endpoint with its expectation in the body, so the host
# compares its own capture against the committed bytes.
#
# Generated from `tests/compliance/corpus/reconfiguration.json` by
# `cargo test --features conformance_host --test conformance_host -- --ignored
# export_hurl`. The gate `the_committed_hurl_suite_equals_the_generated_suite`
# rebuilds this file in memory and compares: a corpus change that was not
# re-exported fails the gate. Edit the corpus, then re-export.

# reconfiguration-the-establishing-commit-opens-the-next-era
# A committed reconfiguration operation MUST establish the next era, and the cluster MUST serve under it with the weights it names
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "reconfiguration-the-establishing-commit-opens-the-next-era",
  "family": "reconfiguration",
  "theorem": "Eras.theorem10",
  "clause": "A committed reconfiguration operation MUST establish the next era, and the cluster MUST serve under it with the weights it names",
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
    "op": "reconfigure",
    "node": "1:1",
    "system": {
      "kind": "increment",
      "node": "3:1"
    }
  },
  "expect": {
    "deliveries": [
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "0000000200000001000000000000000000000003020000000000000003000000010203000300010000000000000002"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000200000001000000000000000000000003020000000000000003000000010203000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000030000000100000000000000000000000303"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000030000000100000000000000000000000303"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "0000000400000001000000000000000000000003040000000000000003"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000400000001000000000000000000000003040000000000000003"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 0,
        "accepted": 3,
        "committed": 3,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "increment 3:1"
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
        "accepted": 3,
        "committed": 3,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "increment 3:1"
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
        "accepted": 3,
        "committed": 3,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "increment 3:1"
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
jsonpath "$.id" == "reconfiguration-the-establishing-commit-opens-the-next-era"
jsonpath "$.family" == "reconfiguration"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 6

# reconfiguration-a-join-inserts-a-weight-zero-learner
# A join MUST insert the new member at weight zero: the learner holds a succession position and is invisible to every majority
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "reconfiguration-a-join-inserts-a-weight-zero-learner",
  "family": "reconfiguration",
  "theorem": "IdentityLaw.weight0_majority_irrelevant",
  "clause": "A join MUST insert the new member at weight zero: the learner holds a succession position and is invisible to every majority",
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
    "op": "reconfigure",
    "node": "1:1",
    "system": {
      "kind": "join",
      "node": "4:1",
      "position": 3
    }
  },
  "expect": {
    "deliveries": [
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000020000000100000000000000000000000302000000000000000300000001020700040001000000030000000000000002"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000020000000100000000000000000000000302000000000000000300000001020700040001000000030000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000030000000100000000000000000000000303"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000030000000100000000000000000000000303"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "0000000400000001000000000000000000000003040000000000000003"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000400000001000000000000000000000003040000000000000003"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 0,
        "accepted": 3,
        "committed": 3,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "join 4:1 at 3"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1",
          "4:1"
        ],
        "weights": [
          1,
          1,
          1,
          0
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
        "accepted": 3,
        "committed": 3,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "join 4:1 at 3"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1",
          "4:1"
        ],
        "weights": [
          1,
          1,
          1,
          0
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
        "accepted": 3,
        "committed": 3,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "join 4:1 at 3"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1",
          "4:1"
        ],
        "weights": [
          1,
          1,
          1,
          0
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
jsonpath "$.id" == "reconfiguration-a-join-inserts-a-weight-zero-learner"
jsonpath "$.family" == "reconfiguration"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 6

# reconfiguration-the-rider-walks-the-view-across-the-slabs-eras
# A commit-time view advance MUST cross more than one era exactly when the pair's own evidence grounds it: the rider names the running view, every era the jump spans is established by a slot the committed frontier covers, and the whole advance lands in one transition with the serving view at the slab's own final era
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "reconfiguration-the-rider-walks-the-view-across-the-slabs-eras",
  "family": "reconfiguration",
  "theorem": "ViewJump.view_jump_safe",
  "clause": "A commit-time view advance MUST cross more than one era exactly when the pair's own evidence grounds it: the rider names the running view, every era the jump spans is established by a slot the committed frontier covers, and the whole advance lands in one transition with the serving view at the slab's own final era",
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
      "op": "deliver",
      "to": "2:1",
      "from": "1:1",
      "wire": "0000000e000000010000000000000000000000030e0000000307000400010000000303000400010a0000000000000001"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "2:1",
    "from": "1:1",
    "wire": "0000000400000001000000000000000000000005040000000000000005"
  },
  "expect": {
    "deliveries": [
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "0000000f000000010000000000000000000000050f00000003000000000000000300000000000000040000000000000005"
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
        "era": 3,
        "view": 1,
        "accepted": 5,
        "committed": 5,
        "applied": 5,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "join 4:1 at 3",
          "increment 4:1",
          "nominate from=0 offset=1"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1",
          "4:1"
        ],
        "weights": [
          1,
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
jsonpath "$.id" == "reconfiguration-the-rider-walks-the-view-across-the-slabs-eras"
jsonpath "$.family" == "reconfiguration"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# reconfiguration-without-the-matching-rider-the-tail-defers
# The same slab with no rider naming the running view MUST NOT walk: the fold stops before the run the serving view cannot carry, the committed frontier lands on the head slot, and the view stands where the pair left it
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "reconfiguration-without-the-matching-rider-the-tail-defers",
  "family": "reconfiguration",
  "theorem": "ViewJump.successor_not_required",
  "clause": "The same slab with no rider naming the running view MUST NOT walk: the fold stops before the run the serving view cannot carry, the committed frontier lands on the head slot, and the view stands where the pair left it",
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
      "op": "deliver",
      "to": "2:1",
      "from": "1:1",
      "wire": "0000000e000000010000000000000000000000030e0000000307000400010000000303000400010a0000000100000001"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "2:1",
    "from": "1:1",
    "wire": "0000000400000001000000000000000000000005040000000000000005"
  },
  "expect": {
    "deliveries": [
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "0000000f000000010000000000000000000000050f00000003000000000000000300000000000000040000000000000005"
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
        "accepted": 5,
        "committed": 3,
        "applied": 3,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "join 4:1 at 3",
          "increment 4:1",
          "nominate from=1 offset=1"
        ],
        "members": [
          "1:1",
          "2:1",
          "3:1",
          "4:1"
        ],
        "weights": [
          1,
          1,
          1,
          0
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
jsonpath "$.id" == "reconfiguration-without-the-matching-rider-the-tail-defers"
jsonpath "$.family" == "reconfiguration"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1
