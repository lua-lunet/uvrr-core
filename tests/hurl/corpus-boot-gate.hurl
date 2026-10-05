# The `boot-gate` family of the uVRR compliance corpus, replayed over the
# transport (`docs/uvrr-host-compliance.md` §8): 8 cases, each POSTed
# to the host's case endpoint with its expectation in the body, so the host
# compares its own capture against the committed bytes.
#
# Generated from `tests/compliance/corpus/boot-gate.json` by
# `cargo test --features conformance_host --test conformance_host -- --ignored
# export_hurl`. The gate `the_committed_hurl_suite_equals_the_generated_suite`
# rebuilds this file in memory and compares: a corpus change that was not
# re-exported fails the gate. Edit the corpus, then re-export.

# boot-gate-the-controlled-halt-is-two-rounds-with-the-drain-between
# The controlled shutdown MUST write Stopping to every marker copy, force the drain strictly between the rounds, and write Stopped to every copy
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "boot-gate-the-controlled-halt-is-two-rounds-with-the-drain-between",
  "family": "boot-gate",
  "theorem": "ReincarnationGeneral.forced_run_terminal_flushed",
  "clause": "The controlled shutdown MUST write Stopping to every marker copy, force the drain strictly between the rounds, and write Stopped to every copy",
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
    "op": "halt",
    "node": "1:1"
  },
  "expect": {
    "deliveries": [],
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
          "Joining@1:1",
          "Stopping@1:1",
          "drain",
          "Stopped@1:1"
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
jsonpath "$.id" == "boot-gate-the-controlled-halt-is-two-rounds-with-the-drain-between"
jsonpath "$.family" == "boot-gate"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# boot-gate-the-clean-restart-continues-the-same-identity
# The next boot over a stopped quorum MUST continue the same identity and reopen Restarting, one marker round at the continuing pair
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "boot-gate-the-clean-restart-continues-the-same-identity",
  "family": "boot-gate",
  "theorem": "ReincarnationGeneral.forced_run_committed",
  "clause": "The next boot over a stopped quorum MUST continue the same identity and reopen Restarting, one marker round at the continuing pair",
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
      "op": "halt",
      "node": "1:1"
    }
  ],
  "input": {
    "op": "restart",
    "node": "1:1",
    "kind": "clean"
  },
  "expect": {
    "deliveries": [],
    "post": [
      {
        "node": "1:1",
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
        "markers": [
          "Restarting@1:1"
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
jsonpath "$.id" == "boot-gate-the-clean-restart-continues-the-same-identity"
jsonpath "$.family" == "boot-gate"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# boot-gate-the-crashed-restart-bumps-and-enters-the-fence
# A crashed reopen MUST read no stopped quorum, bump the identity exactly one life, and reopen fenced: the pair is decided before the wire phase, and the durable Joining round at the bumped pair defers to the seated witness
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "boot-gate-the-crashed-restart-bumps-and-enters-the-fence",
  "family": "boot-gate",
  "theorem": "IdentityLaw.wire_requires_bump",
  "clause": "A crashed reopen MUST read no stopped quorum, bump the identity exactly one life, and reopen fenced: the pair is decided before the wire phase, and the durable Joining round at the bumped pair defers to the seated witness",
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
    }
  ],
  "input": {
    "op": "restart",
    "node": "3:1",
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
jsonpath "$.id" == "boot-gate-the-crashed-restart-bumps-and-enters-the-fence"
jsonpath "$.family" == "boot-gate"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# boot-gate-the-bumped-fence-ticks-silently
# A bumped identity is outside every configuration it can name: its tick MUST NOT vote, MUST NOT drive a view change, and MUST NOT emit any datagram, and nothing resets to zero
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "boot-gate-the-bumped-fence-ticks-silently",
  "family": "boot-gate",
  "theorem": "ReincarnationGeneral.evicted_never_voting",
  "clause": "A bumped identity is outside every configuration it can name: its tick MUST NOT vote, MUST NOT drive a view change, and MUST NOT emit any datagram, and nothing resets to zero",
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
    "op": "tick",
    "node": "3:2"
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
jsonpath "$.id" == "boot-gate-the-bumped-fence-ticks-silently"
jsonpath "$.family" == "boot-gate"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# boot-gate-a-fabricated-vote-from-a-non-member-is-discarded
# A fabricated vote from a non-member MUST be discarded by name: no frontier moves and no datagram is emitted
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "boot-gate-a-fabricated-vote-from-a-non-member-is-discarded",
  "family": "boot-gate",
  "theorem": "ReincarnationGeneral.evicted_never_voting",
  "clause": "A fabricated vote from a non-member MUST be discarded by name: no frontier moves and no datagram is emitted",
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
    "to": "1:1",
    "from": "4:1",
    "wire": "000000030000000100000000000000000000000303"
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
jsonpath "$.id" == "boot-gate-a-fabricated-vote-from-a-non-member-is-discarded"
jsonpath "$.family" == "boot-gate"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# boot-gate-the-first-life-anchors-before-it-answers
# A first life MUST latch its anchor, one Joining round at the one-indexed pair, before it answers anything
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "boot-gate-the-first-life-anchors-before-it-answers",
  "family": "boot-gate",
  "theorem": "IdentityLaw.lawful_one_indexed",
  "clause": "A first life MUST latch its anchor, one Joining round at the one-indexed pair, before it answers anything",
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
    "op": "boot",
    "node": "4:1"
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
      },
      {
        "node": "4:1",
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
        "markers": [
          "Joining@4:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "boot-gate-the-first-life-anchors-before-it-answers"
jsonpath "$.family" == "boot-gate"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# boot-gate-a-fenced-member-adopts-the-commit-bootstrap
# A fenced member of the view's own configuration MUST leave the fence on a Commit announcement from that view's primary: the receiver reopens Normal and no frontier of commitment moves
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "boot-gate-a-fenced-member-adopts-the-commit-bootstrap",
  "family": "boot-gate",
  "theorem": "Acceptor.accept_preserves",
  "clause": "A fenced member of the view's own configuration MUST leave the fence on a Commit announcement from that view's primary: the receiver reopens Normal and no frontier of commitment moves",
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
    "wire": "0000000400000001000000000000000000000002040000000000000002"
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
          "Restarting@3:1"
        ],
        "witnesses": []
      }
    ]
  }
}
HTTP 200
[Asserts]
jsonpath "$.id" == "boot-gate-a-fenced-member-adopts-the-commit-bootstrap"
jsonpath "$.family" == "boot-gate"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0

# boot-gate-an-outside-identity-stays-fenced-on-the-commit
# A fenced identity holding no weight in the view's era MUST stay fenced on the same announcement: it adopts no view, no frontier of commitment moves, and it answers nothing
POST {{base_url}}/case
Content-Type: application/json
{
  "id": "boot-gate-an-outside-identity-stays-fenced-on-the-commit",
  "family": "boot-gate",
  "theorem": "ReincarnationGeneral.evicted_never_voting",
  "clause": "A fenced identity holding no weight in the view's era MUST stay fenced on the same announcement: it adopts no view, no frontier of commitment moves, and it answers nothing",
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
    "to": "3:2",
    "from": "1:1",
    "wire": "0000000400000001000000000000000000000002040000000000000002"
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
jsonpath "$.id" == "boot-gate-an-outside-identity-stays-fenced-on-the-commit"
jsonpath "$.family" == "boot-gate"
jsonpath "$.verdict" == "pass"
jsonpath "$.mismatch" not exists
jsonpath "$.expect.deliveries" count == 0
