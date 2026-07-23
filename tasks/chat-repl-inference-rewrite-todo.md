# Chat/REPL/Inference Rewrite — Todo

## Phase 1: Foundation (R1–R4)

- [ ] R1: Re-export `ChatMessage` from `hkask-types` public API
- [ ] R2: Add `generate_with_messages` to `InferencePort` trait with default impl
- [ ] R3: Implement `generate_with_messages` in `InferenceRouter`
- [ ] R4: Add `build_chat_request_messages` to `chat_protocol`

## Phase 2: Backends (R5)

- [ ] R5: Update each backend to use message array in request body

## Phase 3: Service Layer (R6–R9)

- [ ] R6: Add `thread_history_messages` to `ThreadRegistry`
- [ ] R7: Rewrite `ChatService::prepare_chat` for message arrays
- [ ] R8: Rewrite `ChatService::chat` to use `generate_with_messages`
- [ ] R9: Rewrite `ChatService::execute_turn` for message arrays

## Phase 4: Entry Points (R10–R11)

- [ ] R10: Update `chat.rs` CLI streaming path
- [ ] R11: Update REPL turn loop for message arrays

## Phase 5: Validation (R12)

- [ ] R12: Integration test — multi-turn no echo