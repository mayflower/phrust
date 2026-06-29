# OSR Entry Maps

OSR entry maps describe where a future optimized tier could enter a hot loop
from the interpreter. They are metadata only: no transfer, no native entry, and
no default-on behavior is introduced.

## Loop-Header Detection

Dense bytecode OSR candidates are detected from CFG terminators. A branch whose
target block is at or before the current block is treated as a loop backedge,
and the target becomes the loop header. The entry keeps:

- function index;
- loop header block and bytecode instruction offset;
- backedge block as a fake control predecessor;
- source span for the header instruction when available;
- required live VM slots.

The fake predecessor is report metadata for future optimizers. It does not
modify dense bytecode or interpreter control flow.

## Entry Maps

Each entry records live local, register, and iterator slots with an abstract VM
target location, value class, and reference/COW safety classification. The first
implementation is deliberately conservative: value classes are generally
unknown, and slots are guarded as VM-owned values rather than native registers.

Counters use the shared performance-report naming style:

- `osr_entry_candidates`
- `osr_entry_representable`
- `osr_entry_rejected_by_reason`
- `osr_live_slots`

## Unsupported PHP State

OSR is rejected when the loop body contains state that is not yet modeled:

- dynamic calls;
- foreach iterator state;
- by-reference foreach or other reference/COW-sensitive state;
- array mutation/COW state;
- output state.

The map also accepts explicit annotations for states that may be known before
dense bytecode has dedicated opcodes: try/finally, generators, fibers, and
by-reference foreach. Those annotations let future frontend or runtime analyses
explain rejection without pretending the loop is representable.

Packed by-value foreach loops remain rejected until iterator state has a stable
entry model. By-reference foreach is always rejected by the current metadata
because alias cells and lingering reference semantics are not captured.

## Region IR Entries

Region IR already has `Entry(EntryId)` nodes. The OSR metadata pass collects
those nodes, records their fake control predecessor, and treats `Param` inputs
as live VM slots. Pure scalar floating nodes may move across an OSR entry only
when their dependencies are available at entry. Control, memory, guard,
snapshot, call, output, reference/COW, and deopt-sensitive nodes remain pinned
or rejected.

## Guards, Snapshots, And Resume Metadata

OSR entries are related to guard/snapshot/resume metadata but are not exits.
The entry map says what the interpreter must provide at a loop header. Guard and
side-exit metadata say how an optimized path returns to the interpreter after a
speculation fails. Future executable tiers need both directions before enabling
OSR transfer.
