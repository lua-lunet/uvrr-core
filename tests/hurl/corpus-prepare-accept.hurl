# The `prepare-accept` family of the uVRR compliance corpus, replayed over the
# transport (`docs/uvrr-host-compliance.md` §8): 52 cases, each POSTed
# to the host's case endpoint with its expectation in the body, so the host
# compares its own capture against the committed bytes.
#
# Generated from `tests/compliance/corpus/prepare-accept.json` by
# `cargo test --features conformance_host --test conformance_host -- --ignored
# export_hurl`. The gate `the_committed_hurl_suite_equals_the_generated_suite`
# rebuilds this file in memory and compares: a corpus change that was not
# re-exported fails the gate. Edit the corpus, then re-export.

# prepare-accept-boot-normal-view-less-slot-less-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-less-slot-less-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "1:1",
    "wire": "0000000200000001000000000000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000001"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-less-slot-less-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-normal-view-less-slot-less-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-less-slot-less-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "1:1",
    "wire": "0000000200000001000000000000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000002"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-less-slot-less-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-normal-view-less-slot-less-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-less-slot-less-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "1:1",
    "wire": "0000000200000001000000000000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000003"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-less-slot-less-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-normal-view-less-slot-equal-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-less-slot-equal-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "1:1",
    "wire": "00000002000000010000000000000000000000030200000000000000030000000101000000007665637400000000000000040000000870726f706f7365640000000000000001"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-less-slot-equal-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-normal-view-less-slot-equal-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-less-slot-equal-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "1:1",
    "wire": "00000002000000010000000000000000000000030200000000000000030000000101000000007665637400000000000000050000000870726f706f7365640000000000000002"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-less-slot-equal-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-normal-view-less-slot-equal-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-less-slot-equal-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "1:1",
    "wire": "00000002000000010000000000000000000000030200000000000000030000000101000000007665637400000000000000060000000870726f706f7365640000000000000003"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-less-slot-equal-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-normal-view-less-slot-greater-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-less-slot-greater-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "1:1",
    "wire": "00000002000000010000000000000000000000040200000000000000040000000101000000007665637400000000000000070000000870726f706f7365640000000000000001"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-less-slot-greater-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-normal-view-less-slot-greater-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-less-slot-greater-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "1:1",
    "wire": "00000002000000010000000000000000000000040200000000000000040000000101000000007665637400000000000000080000000870726f706f7365640000000000000002"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-less-slot-greater-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-normal-view-less-slot-greater-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-less-slot-greater-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "1:1",
    "wire": "00000002000000010000000000000000000000040200000000000000040000000101000000007665637400000000000000090000000870726f706f7365640000000000000003"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-less-slot-greater-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-normal-view-equal-slot-less-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-equal-slot-less-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.report_last",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "2:1",
    "wire": "0000000200000001000000010000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000001"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000203"
      }
    ],
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-equal-slot-less-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-normal-view-equal-slot-less-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-equal-slot-less-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.report_last",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "2:1",
    "wire": "0000000200000001000000010000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000002"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000203"
      }
    ],
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-equal-slot-less-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-normal-view-equal-slot-less-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-equal-slot-less-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.report_last",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "2:1",
    "wire": "0000000200000001000000010000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000003"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000203"
      }
    ],
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-equal-slot-less-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-normal-view-equal-slot-equal-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-equal-slot-equal-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.accept_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "2:1",
    "wire": "000000020000000100000001000000000000000302000000000000000300000001010000000076656374000000000000000d0000000870726f706f7365640000000000000001"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000303"
      }
    ],
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
        "accepted": 3,
        "committed": 2,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "proposed"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-equal-slot-equal-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-normal-view-equal-slot-equal-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-equal-slot-equal-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.accept_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "2:1",
    "wire": "000000020000000100000001000000000000000302000000000000000300000001010000000076656374000000000000000e0000000870726f706f7365640000000000000002"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000303"
      }
    ],
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
        "accepted": 3,
        "committed": 2,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "proposed"
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-equal-slot-equal-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-normal-view-equal-slot-greater-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-equal-slot-greater-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.reachable_inv",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "2:1",
    "wire": "000000020000000100000001000000000000000402000000000000000400000001010000000076656374000000000000000f0000000870726f706f7365640000000000000001"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000900000001000000010000000000000002090000000000000003"
      }
    ],
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-equal-slot-greater-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-normal-view-equal-slot-greater-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-equal-slot-greater-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.reachable_inv",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "2:1",
    "wire": "00000002000000010000000100000000000000040200000000000000040000000101000000007665637400000000000000100000000870726f706f7365640000000000000002"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000900000001000000010000000000000002090000000000000003"
      }
    ],
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-equal-slot-greater-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-normal-view-equal-slot-greater-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-equal-slot-greater-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.reachable_inv",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "2:1",
    "wire": "00000002000000010000000100000000000000040200000000000000040000000101000000007665637400000000000000110000000870726f706f7365640000000000000003"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000900000001000000010000000000000002090000000000000003"
      }
    ],
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-equal-slot-greater-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-normal-view-greater-slot-less-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-greater-slot-less-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.promise_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "3:1",
    "wire": "0000000200000001000000020000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000001"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "3:1",
        "wire": "0000000900000001000000020000000000000002090000000000000003"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 2,
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
        "view": 2,
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
        "view": 2,
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-greater-slot-less-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 11

# prepare-accept-boot-normal-view-greater-slot-less-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-greater-slot-less-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.promise_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "3:1",
    "wire": "0000000200000001000000020000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000002"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "3:1",
        "wire": "0000000900000001000000020000000000000002090000000000000003"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 2,
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
        "view": 2,
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
        "view": 2,
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-greater-slot-less-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 11

# prepare-accept-boot-normal-view-greater-slot-less-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-greater-slot-less-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.promise_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "3:1",
    "wire": "0000000200000001000000020000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000003"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "3:1",
        "wire": "0000000900000001000000020000000000000002090000000000000003"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 2,
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
        "view": 2,
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
        "view": 2,
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-greater-slot-less-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 11

# prepare-accept-boot-normal-view-greater-slot-equal-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-greater-slot-equal-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.promise_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "3:1",
    "wire": "00000002000000010000000200000000000000030200000000000000030000000101000000007665637400000000000000150000000870726f706f7365640000000000000001"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "3:1",
        "wire": "0000000900000001000000020000000000000002090000000000000003"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 2,
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
        "view": 2,
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
        "view": 2,
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-greater-slot-equal-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 11

# prepare-accept-boot-normal-view-greater-slot-equal-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-greater-slot-equal-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.promise_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "3:1",
    "wire": "00000002000000010000000200000000000000030200000000000000030000000101000000007665637400000000000000160000000870726f706f7365640000000000000002"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "3:1",
        "wire": "0000000900000001000000020000000000000002090000000000000003"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 2,
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
        "view": 2,
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
        "view": 2,
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-greater-slot-equal-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 11

# prepare-accept-boot-normal-view-greater-slot-equal-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-greater-slot-equal-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.promise_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "3:1",
    "wire": "00000002000000010000000200000000000000030200000000000000030000000101000000007665637400000000000000170000000870726f706f7365640000000000000003"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "3:1",
        "wire": "0000000900000001000000020000000000000002090000000000000003"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 2,
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
        "view": 2,
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
        "view": 2,
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-greater-slot-equal-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 11

# prepare-accept-boot-normal-view-greater-slot-greater-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-greater-slot-greater-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.promise_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "3:1",
    "wire": "00000002000000010000000200000000000000040200000000000000040000000101000000007665637400000000000000180000000870726f706f7365640000000000000001"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "3:1",
        "wire": "0000000900000001000000020000000000000002090000000000000003"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 2,
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
        "view": 2,
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
        "view": 2,
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-greater-slot-greater-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 11

# prepare-accept-boot-normal-view-greater-slot-greater-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-greater-slot-greater-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.promise_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "3:1",
    "wire": "00000002000000010000000200000000000000040200000000000000040000000101000000007665637400000000000000190000000870726f706f7365640000000000000002"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "3:1",
        "wire": "0000000900000001000000020000000000000002090000000000000003"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 2,
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
        "view": 2,
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
        "view": 2,
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-greater-slot-greater-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 11

# prepare-accept-boot-normal-view-greater-slot-greater-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-normal-view-greater-slot-greater-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.promise_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    "to": "3:1",
    "from": "3:1",
    "wire": "000000020000000100000002000000000000000402000000000000000400000001010000000076656374000000000000001a0000000870726f706f7365640000000000000003"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "3:1",
        "to": "3:1",
        "wire": "0000000900000001000000020000000000000002090000000000000003"
      },
      {
        "from": "1:1",
        "to": "2:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "1:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "2:1",
        "to": "1:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "000000050000000100000002000000000000000005"
      },
      {
        "from": "2:1",
        "to": "3:1",
        "wire": "0000000600000001000000020000000000000002060000000100000001000000000000000200000000000000020000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010102000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "1:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      },
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000700000001000000020000000000000002070000000200000000000000010000000002010000000000000002000000010202000000030001000100020001000300010000000000000002000000000000000202000000030001000100020001000300010000000000000002"
      }
    ],
    "post": [
      {
        "node": "1:1",
        "status": "Normal",
        "era": 1,
        "view": 2,
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
        "view": 2,
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
        "view": 2,
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
jsonpath "$.id" == "prepare-accept-boot-normal-view-greater-slot-greater-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 11

# prepare-accept-boot-restarting-view-less-slot-less-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-less-slot-less-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "1:1",
    "wire": "0000000200000001000000000000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000001"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-less-slot-less-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-less-slot-less-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-less-slot-less-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "1:1",
    "wire": "0000000200000001000000000000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000002"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-less-slot-less-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-less-slot-less-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-less-slot-less-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "1:1",
    "wire": "0000000200000001000000000000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000003"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-less-slot-less-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-less-slot-equal-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-less-slot-equal-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "1:1",
    "wire": "000000020000000100000000000000000000000302000000000000000300000001010000000076656374000000000000001e0000000870726f706f7365640000000000000001"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-less-slot-equal-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-less-slot-equal-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-less-slot-equal-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "1:1",
    "wire": "000000020000000100000000000000000000000302000000000000000300000001010000000076656374000000000000001f0000000870726f706f7365640000000000000002"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-less-slot-equal-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-less-slot-equal-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-less-slot-equal-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "1:1",
    "wire": "00000002000000010000000000000000000000030200000000000000030000000101000000007665637400000000000000200000000870726f706f7365640000000000000003"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-less-slot-equal-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-less-slot-greater-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-less-slot-greater-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "1:1",
    "wire": "00000002000000010000000000000000000000040200000000000000040000000101000000007665637400000000000000210000000870726f706f7365640000000000000001"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-less-slot-greater-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-less-slot-greater-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-less-slot-greater-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "1:1",
    "wire": "00000002000000010000000000000000000000040200000000000000040000000101000000007665637400000000000000220000000870726f706f7365640000000000000002"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-less-slot-greater-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-less-slot-greater-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-less-slot-greater-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is below its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "1:1",
    "wire": "00000002000000010000000000000000000000040200000000000000040000000101000000007665637400000000000000230000000870726f706f7365640000000000000003"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-less-slot-greater-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-equal-slot-less-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-equal-slot-less-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.report_last",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "2:1",
    "wire": "0000000200000001000000010000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000001"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000203"
      }
    ],
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-equal-slot-less-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-restarting-view-equal-slot-less-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-equal-slot-less-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.report_last",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "2:1",
    "wire": "0000000200000001000000010000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000002"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000203"
      }
    ],
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-equal-slot-less-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-restarting-view-equal-slot-less-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-equal-slot-less-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.report_last",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "2:1",
    "wire": "0000000200000001000000010000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000003"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000203"
      }
    ],
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-equal-slot-less-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-restarting-view-equal-slot-equal-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-equal-slot-equal-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.accept_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "2:1",
    "wire": "00000002000000010000000100000000000000030200000000000000030000000101000000007665637400000000000000270000000870726f706f7365640000000000000001"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000303"
      }
    ],
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
        "accepted": 3,
        "committed": 2,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "proposed"
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-equal-slot-equal-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-restarting-view-equal-slot-equal-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-equal-slot-equal-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.accept_preserves",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "2:1",
    "wire": "00000002000000010000000100000000000000030200000000000000030000000101000000007665637400000000000000280000000870726f706f7365640000000000000002"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "000000030000000100000001000000000000000303"
      }
    ],
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
        "accepted": 3,
        "committed": 2,
        "applied": 2,
        "journal": [
          "void",
          "init order=1:1,2:1,3:1",
          "proposed"
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-equal-slot-equal-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-restarting-view-equal-slot-greater-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-equal-slot-greater-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.reachable_inv",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "2:1",
    "wire": "00000002000000010000000100000000000000040200000000000000040000000101000000007665637400000000000000290000000870726f706f7365640000000000000001"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000900000001000000010000000000000002090000000000000003"
      }
    ],
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-equal-slot-greater-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-restarting-view-equal-slot-greater-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-equal-slot-greater-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.reachable_inv",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "2:1",
    "wire": "000000020000000100000001000000000000000402000000000000000400000001010000000076656374000000000000002a0000000870726f706f7365640000000000000002"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000900000001000000010000000000000002090000000000000003"
      }
    ],
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-equal-slot-greater-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-restarting-view-equal-slot-greater-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-equal-slot-greater-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.reachable_inv",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is equal to its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "2:1",
    "wire": "000000020000000100000001000000000000000402000000000000000400000001010000000076656374000000000000002b0000000870726f706f7365640000000000000003"
  },
  "expect": {
    "deliveries": [
      {
        "from": "3:1",
        "to": "2:1",
        "wire": "0000000900000001000000010000000000000002090000000000000003"
      }
    ],
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-equal-slot-greater-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 1

# prepare-accept-boot-restarting-view-greater-slot-less-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-greater-slot-less-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "3:1",
    "wire": "0000000200000001000000020000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000001"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-greater-slot-less-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-greater-slot-less-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-greater-slot-less-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "3:1",
    "wire": "0000000200000001000000020000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000002"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-greater-slot-less-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-greater-slot-less-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-greater-slot-less-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is below the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "3:1",
    "wire": "0000000200000001000000020000000000000002020000000000000002000000010202000000030001000100020001000300010000000000000003"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-greater-slot-less-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-greater-slot-equal-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-greater-slot-equal-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "3:1",
    "wire": "000000020000000100000002000000000000000302000000000000000300000001010000000076656374000000000000002f0000000870726f706f7365640000000000000001"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-greater-slot-equal-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-greater-slot-equal-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-greater-slot-equal-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "3:1",
    "wire": "00000002000000010000000200000000000000030200000000000000030000000101000000007665637400000000000000300000000870726f706f7365640000000000000002"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-greater-slot-equal-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-greater-slot-equal-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-greater-slot-equal-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is equal to the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "3:1",
    "wire": "00000002000000010000000200000000000000030200000000000000030000000101000000007665637400000000000000310000000870726f706f7365640000000000000003"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-greater-slot-equal-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-greater-slot-greater-piggyback-less
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-greater-slot-greater-piggyback-less",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is below its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "3:1",
    "wire": "00000002000000010000000200000000000000040200000000000000040000000101000000007665637400000000000000320000000870726f706f7365640000000000000001"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-greater-slot-greater-piggyback-less"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-greater-slot-greater-piggyback-equal
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-greater-slot-greater-piggyback-equal",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is equal to its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "3:1",
    "wire": "00000002000000010000000200000000000000040200000000000000040000000101000000007665637400000000000000330000000870726f706f7365640000000000000002"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-greater-slot-greater-piggyback-equal"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# prepare-accept-boot-restarting-view-greater-slot-greater-piggyback-greater
# A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "prepare-accept-boot-restarting-view-greater-slot-greater-piggyback-greater",
  "family": "prepare-accept",
  "theorem": "Acceptor.free_forbids",
  "clause": "A backup MUST take exactly one route on a Prepare whose view is above its own, whose offered slot is above the successor of its accepted frontier, and whose piggybacked commit frontier is above its committed frontier: a mismatch drops, a retransmit re-answers without moving a frontier, an accept journals and answers, a gap holds the frontier and fetches, and a higher view fences the node",
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
      "op": "tick_all"
    },
    {
      "op": "deliver_all"
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
    },
    {
      "op": "halt",
      "node": "3:1"
    },
    {
      "op": "restart",
      "node": "3:1",
      "kind": "clean"
    }
  ],
  "input": {
    "op": "deliver",
    "to": "3:1",
    "from": "3:1",
    "wire": "00000002000000010000000200000000000000040200000000000000040000000101000000007665637400000000000000340000000870726f706f7365640000000000000003"
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
        "status": "Restarting",
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "prepare-accept-boot-restarting-view-greater-slot-greater-piggyback-greater"
jsonpath "$.family" == "prepare-accept"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0
