# The transport contract of `docs/uvrr-host-compliance.md` §8, asserted by
# hand against a live host. The generated `corpus-*.hurl` files replay the
# corpus through the case endpoint of §8.1; this file covers the endpoints
# that replay no case: the health probe, and the session endpoints of §8.2
# driving one case's abstract host operations one at a time.
#
# The session id is captured from the response rather than assumed, so the
# file does not depend on being the first to open a session when Hurl runs
# the suite in parallel.

GET {{base_url}}/health
HTTP 200
[Asserts]
jsonpath "$.host" == "uvrr-conformance"
jsonpath "$.crate" == "uvrr-core"
jsonpath "$.version" exists

# A session is a cluster, and provisioning is what begins one, so the
# session opens with the provision operation's own arguments.
POST {{base_url}}/session
Content-Type: application/json
{
  "op": "provision",
  "nodes": 3,
  "timeout": 3
}
HTTP 200
[Captures]
session: jsonpath "$.session"
[Asserts]
jsonpath "$.session" isString

# The deterministic bootstrap, the abstract host interface's own `settle`.
POST {{base_url}}/session/{{session}}/op
Content-Type: application/json
{
  "op": "settle"
}
HTTP 200
[Asserts]
jsonpath "$.ok" == true

# One proposal, the same input the `agreement` family records.
POST {{base_url}}/session/{{session}}/op
Content-Type: application/json
{
  "op": "propose",
  "node": "1:1",
  "payload": "agreed"
}
HTTP 200
[Asserts]
jsonpath "$.ok" == true

# The settling window: drained to quiet, and carrying the whole delivered
# set. Six datagrams for three members — two Prepares out, two PrepareOks
# back, two Commits out — so a count of six asserts that six and no more
# were delivered, and the captures' completeness is what makes silence
# assertable at all.
GET {{base_url}}/session/{{session}}/capture
HTTP 200
[Asserts]
jsonpath "$.expect.deliveries" count == 6
jsonpath "$.expect.deliveries[0].from" == "1:1"
jsonpath "$.expect.deliveries[0].to" == "2:1"
jsonpath "$.expect.post" count == 3
jsonpath "$.expect.post[0].node" == "1:1"
jsonpath "$.expect.post[0].status" == "Normal"
jsonpath "$.expect.post[0].committed" == 3
jsonpath "$.expect.post[0].journal[2]" == "agreed"
jsonpath "$.expect.post[2].committed" == 3

# An unknown session is a named refusal, not a silent success.
GET {{base_url}}/session/no-such-session/capture
HTTP 404
[Asserts]
jsonpath "$.error" == "no such session"

# The case endpoint takes POST.
GET {{base_url}}/case
HTTP 405
[Asserts]
jsonpath "$.error" exists