/* vrr-core C ABI.
 *
 * Sans-io: no sockets, no threads, no runtime, no callbacks. You hand the node
 * an input and then drain the outputs it produced. Every call is synchronous
 * and returns a status code; nothing is allocated on your behalf except the
 * opaque node handle.
 *
 * Byte-slice arguments are (len, ptr) pairs and are never retained past the
 * call. Integers that do not fit a C type portably (u64 slots, nonces,
 * execution times) cross as decimal ASCII, because LuaJIT numbers are doubles
 * and would silently lose precision above 2^53.
 */
#ifndef VRR_H
#define VRR_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Status codes. Every entry point returns one of these. */
#define VRR_OK            0
#define VRR_INVALID      (-1)  /* null pointer or non-UTF-8 argument        */
#define VRR_CONFIG       (-2)  /* membership rejected (size or ordering)    */
#define VRR_NUMBER       (-3)  /* decimal-ASCII integer did not parse       */
#define VRR_CLIENT_JSON  (-4)  /* client payload is not a valid request     */
#define VRR_VRR_MESSAGE  (-5)  /* peer datagram failed to decode            */
#define VRR_TOO_LARGE    (-6)  /* an output exceeds one datagram; see below  */
#define VRR_SERVICE      (-7)  /* service execution failed                  */
#define VRR_PANIC      (-127)  /* caught panic; the node is now poisoned    */

/* vrr_node_next return values. Note these are NOT status codes: 1 means an
 * output was dequeued and 0 means the queue is empty. */
#define VRR_NEXT_DEQUEUED 1
#define VRR_NEXT_EMPTY    0

/* Output kinds written to out_kind by vrr_node_next. */
#define VRR_OUT_BROADCAST 1  /* send to every peer; out_to is unused  */
#define VRR_OUT_SEND      2  /* send to the peer in out_to            */
#define VRR_OUT_REPLY     3  /* client reply; header out-params are 0 */

/* Largest datagram the node will emit or accept. A whole-log transfer
 * (DO_EPOCH_CHANGE / START_EPOCH / RECOVERY_RESPONSE) that would exceed this
 * is refused with VRR_TOO_LARGE, and the refusal is atomic: the node's state
 * is rolled back and it keeps serving. It cannot complete that transfer while
 * the log is moved whole, so treat VRR_TOO_LARGE on those tags as "this node
 * needs catch-up by some means other than one datagram". */
#define VRR_MAX_DATAGRAM 65507

/* Opaque node handle. */
typedef void vrr_node_t;

/* Creates a node. `members` is a NUL-separated (0x00), strictly sorted list of
 * at least three node names; `own` must be one of them. Writes the handle to
 * *out. Node ids are indices into that sorted list, so every replica must be
 * given a byte-identical membership. */
int32_t vrr_node_new(size_t members_len, const uint8_t *members, size_t own_len,
                     const uint8_t *own, void **out);

void vrr_node_free(void *node);

/* Submits a client request. `execution_time` is decimal ASCII. Refused
 * silently (VRR_OK, no output) unless this node is a Normal leader. */
int32_t vrr_node_request(void *node, size_t execution_time_len,
                         const uint8_t *execution_time, size_t json_len,
                         const uint8_t *json);

/* Delivers one peer datagram: a 16-byte big-endian header
 * (tag u32, epoch u32, slot u64) followed by the JSON body. */
int32_t vrr_node_receive(void *node, uint32_t from, size_t len,
                         const uint8_t *data);

/* Clock ticks. `idle` makes a Normal leader heartbeat; `leader_timeout` starts
 * an epoch change. The host owns all timing. */
int32_t vrr_node_idle(void *node);
int32_t vrr_node_leader_timeout(void *node);

/* Starts recovery after a restart. `nonce` is decimal ASCII and MUST NOT
 * repeat across restarts of this node — it is the only thing distinguishing a
 * fresh recovery from a stale in-flight one. Call this if and only if the node
 * really did restart: recovering at first boot deadlocks a cold cluster (no
 * peer is Normal to answer), and not recovering after a restart lets an
 * amnesiac node acknowledge entries it never held. */
int32_t vrr_node_recover(void *node, size_t nonce_len, const uint8_t *nonce);

/* Dequeues one queued output. Writes VRR_OUT_* to out_kind and the body into
 * out_data (at most `capacity` bytes), with its length in out_len. The u64 slot
 * arrives split as out_slot_hi/out_slot_lo.
 *
 * Returns VRR_NEXT_DEQUEUED (1) having consumed one output, VRR_NEXT_EMPTY (0)
 * when the queue is empty, or a negative status. On VRR_TOO_LARGE the output is
 * left on the queue and *out_len already holds the length needed, so a caller
 * may resize and retry. Drain in a loop until 0. */
int32_t vrr_node_next(void *node, uint32_t *out_kind, uint32_t *out_to,
                      uint32_t *out_tag, uint32_t *out_epoch,
                      uint32_t *out_slot_hi, uint32_t *out_slot_lo,
                      size_t capacity, size_t *out_len, uint8_t *out_data);

#ifdef __cplusplus
}
#endif

#endif /* VRR_H */
