# Assert nonempty results in equivalence tests

## What happened

- A region-search worker test compared two empty lists because its minimum size exceeded every object in the fixture. The comparison passed without establishing that the worker returned the expected objects.

## Why

- Equivalent empty results can hide a broken implementation or a fixture that never exercises the intended path.

## Do this instead

- Choose fixture inputs that produce a meaningful result and assert that precondition before comparing the worker result with the synchronous result.
- When testing cancellation, make the cancelled result observably different from the result already installed.

Decided in: isk-6ptr
