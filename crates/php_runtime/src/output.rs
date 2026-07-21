//! Byte-oriented output buffering.

use std::{collections::BTreeMap, fmt, sync::Arc};

use crate::{context::RuntimeHttpResponseState, string::PhpString};

/// Fixed root coalescing and delivery chunk bound. Nested PHP output buffers
/// remain application-controlled; only the transport-facing root is bounded.
pub const OUTPUT_CHUNK_BYTES: usize = 32 * 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutputDeliveryError {
    message: String,
}

impl OutputDeliveryError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Synchronous request-local output destination. A server implementation may
/// block in `write` to propagate bounded-channel backpressure to its dedicated
/// PHP worker.
pub trait OutputSink: Send + Sync + 'static {
    /// Whether the first root chunk must commit the HTTP response head before
    /// delivery. HEAD-style counting sinks return false so the final produced
    /// length remains available without retaining body bytes.
    fn commit_before_write(&self) -> bool {
        true
    }

    fn commit(
        &self,
        response: &RuntimeHttpResponseState,
        complete_length: Option<u64>,
    ) -> Result<(), OutputDeliveryError>;
    fn write(&self, chunk: Vec<u8>) -> Result<(), OutputDeliveryError>;
    fn finish(&self) -> Result<(), OutputDeliveryError> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct OutputSinkHandle(Arc<dyn OutputSink>);

impl OutputSinkHandle {
    #[must_use]
    pub fn new(sink: impl OutputSink) -> Self {
        Self(Arc::new(sink))
    }
}

impl fmt::Debug for OutputSinkHandle {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("OutputSinkHandle")
    }
}

impl PartialEq for OutputSinkHandle {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for OutputSinkHandle {}

/// Runtime output buffer.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OutputStats {
    /// Final/root-visible bytes are computed by the VM from `OutputBuffer::len`.
    pub appends: u64,
    /// Writes that appended more than one slice after one reserve.
    pub batch_writes: u64,
    /// Fast-path appends that batched adjacent exact-output slices.
    pub batched_appends: u64,
    /// Exact bytes written by batched adjacent-output appends.
    pub batch_bytes: u64,
    /// Active output-buffer flushes into a parent or root buffer.
    pub flushes: u64,
    /// Appends that used a VM-proven exact-output fast path.
    pub fast_appends: u64,
    /// Generic conversion/output appends grouped by stable fallback reason.
    pub slow_appends_by_reason: BTreeMap<String, u64>,
    pub produced_bytes: u64,
    pub delivered_bytes: u64,
    pub delivered_chunks: u64,
    pub max_root_pending_bytes: usize,
    pub delivery_failed: bool,
}

/// Runtime output buffer.
#[derive(Clone, Debug)]
pub struct OutputBuffer {
    /// Complete captured output for collecting callers, or bounded pending
    /// root bytes for streaming callers.
    bytes: Vec<u8>,
    stack: Vec<Vec<u8>>,
    stats: OutputStats,
    sink: Option<OutputSinkHandle>,
    delivery_error: Option<OutputDeliveryError>,
    response: RuntimeHttpResponseState,
    head_committed: bool,
    sink_finished: bool,
}

impl PartialEq for OutputBuffer {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes && self.stack == other.stack
    }
}

impl Eq for OutputBuffer {}

impl Default for OutputBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl OutputBuffer {
    /// Creates an empty output buffer.
    #[must_use]
    pub fn new() -> Self {
        Self {
            bytes: Vec::new(),
            stack: Vec::new(),
            stats: OutputStats::default(),
            sink: None,
            delivery_error: None,
            response: RuntimeHttpResponseState::default(),
            head_committed: false,
            sink_finished: false,
        }
    }

    /// Creates an empty output buffer with root buffer capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            bytes: Vec::with_capacity(capacity),
            stack: Vec::new(),
            stats: OutputStats::default(),
            sink: None,
            delivery_error: None,
            response: RuntimeHttpResponseState::default(),
            head_committed: false,
            sink_finished: false,
        }
    }

    #[must_use]
    pub fn with_sink(sink: OutputSinkHandle) -> Self {
        Self {
            sink: Some(sink),
            bytes: Vec::with_capacity(OUTPUT_CHUNK_BYTES),
            ..Self::new()
        }
    }

    /// Returns request-local output write statistics.
    #[must_use]
    pub fn stats(&self) -> OutputStats {
        self.stats.clone()
    }

    /// Reserves capacity in the active buffer.
    pub fn reserve(&mut self, additional: usize) {
        if additional == 0 {
            return;
        }
        if let Some(buffer) = self.stack.last_mut() {
            buffer.reserve(additional);
        } else if self.sink.is_some() {
            self.bytes
                .reserve(additional.min(OUTPUT_CHUNK_BYTES.saturating_sub(self.bytes.len())));
        } else {
            self.bytes.reserve(additional);
        }
    }

    /// Appends exact bytes.
    pub fn write_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        let bytes = bytes.as_ref();
        if bytes.is_empty() {
            return;
        }
        self.stats.appends += 1;
        if let Some(buffer) = self.stack.last_mut() {
            buffer.extend_from_slice(bytes);
        } else {
            self.append_root(bytes);
        }
    }

    /// Appends exact bytes through a VM-proven fast path.
    pub fn write_fast_bytes(&mut self, bytes: impl AsRef<[u8]>) {
        let bytes = bytes.as_ref();
        if bytes.is_empty() {
            return;
        }
        self.stats.fast_appends += 1;
        self.write_bytes(bytes);
    }

    /// Appends several byte slices with one active-buffer reservation.
    pub fn write_slices(&mut self, slices: &[&[u8]]) {
        let total = slices.iter().map(|bytes| bytes.len()).sum::<usize>();
        if total == 0 {
            return;
        }
        self.stats.appends += 1;
        if slices
            .iter()
            .filter(|bytes| !bytes.is_empty())
            .take(2)
            .count()
            > 1
        {
            self.stats.batch_writes += 1;
            self.stats.batched_appends += 1;
            self.stats.batch_bytes += total as u64;
        }
        if let Some(buffer) = self.stack.last_mut() {
            buffer.reserve(total);
            for bytes in slices.iter().copied().filter(|bytes| !bytes.is_empty()) {
                buffer.extend_from_slice(bytes);
            }
        } else {
            for bytes in slices.iter().copied().filter(|bytes| !bytes.is_empty()) {
                self.append_root(bytes);
            }
        }
    }

    /// Appends several byte slices through a VM-proven fast path.
    pub fn write_fast_slices(&mut self, slices: &[&[u8]]) {
        let has_bytes = slices.iter().any(|bytes| !bytes.is_empty());
        if !has_bytes {
            return;
        }
        self.stats.fast_appends += 1;
        self.write_slices(slices);
    }

    /// Appends a PHP string's exact bytes.
    pub fn write_php_string(&mut self, value: &PhpString) {
        self.write_bytes(value.as_bytes());
    }

    /// Appends a PHP string's exact bytes through a VM-proven fast path.
    pub fn write_fast_php_string(&mut self, value: &PhpString) {
        self.write_fast_bytes(value.as_bytes());
    }

    /// Records that output had to use a generic conversion/fallback path.
    pub fn record_slow_append_reason(&mut self, reason: &'static str) {
        *self
            .stats
            .slow_appends_by_reason
            .entry(reason.to_string())
            .or_default() += 1;
    }

    /// Convenience for tests and ASCII literals.
    pub fn write_test_str(&mut self, text: &str) {
        self.write_bytes(text.as_bytes());
    }

    /// Returns the exact output bytes.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Returns the exact buffered byte count.
    #[must_use]
    pub fn len(&self) -> usize {
        self.stats.produced_bytes.min(usize::MAX as u64) as usize
    }

    /// Returns the root-visible byte count plus bytes captured by active PHP
    /// output-buffering levels.
    #[must_use]
    pub fn total_len(&self) -> usize {
        self.len() + self.stack.iter().map(Vec::len).sum::<usize>()
    }

    /// Returns true when no output bytes have been buffered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.stats.produced_bytes == 0
    }

    /// Consumes the buffer and returns exact output bytes.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        if self.sink.is_some() {
            Vec::new()
        } else {
            self.bytes
        }
    }

    /// Test/debug convenience for textual assertions.
    #[must_use]
    pub fn to_string_lossy(&self) -> String {
        String::from_utf8_lossy(&self.bytes).into_owned()
    }

    /// Clears buffered output.
    pub fn clear(&mut self) {
        self.bytes.clear();
        self.stack.clear();
        self.stats.produced_bytes = 0;
        self.stats.delivered_bytes = 0;
        self.stats.delivered_chunks = 0;
        self.stats.max_root_pending_bytes = 0;
    }

    /// Starts a nested PHP output buffer.
    pub fn start_buffer(&mut self) {
        self.stack.push(Vec::new());
    }

    /// Returns the current output buffering level.
    #[must_use]
    pub fn buffer_level(&self) -> usize {
        self.stack.len()
    }

    /// Returns the active buffer contents, if output buffering is active.
    #[must_use]
    pub fn current_buffer_bytes(&self) -> Option<&[u8]> {
        self.stack.last().map(Vec::as_slice)
    }

    /// Returns the active buffer length, if output buffering is active.
    #[must_use]
    pub fn current_buffer_len(&self) -> Option<usize> {
        self.stack.last().map(Vec::len)
    }

    /// Discards and returns the active buffer.
    pub fn pop_buffer_clean(&mut self) -> Option<Vec<u8>> {
        self.stack.pop()
    }

    /// Flushes the active buffer into its parent buffer or root output.
    pub fn pop_buffer_flush(&mut self) -> Option<()> {
        let bytes = self.stack.pop()?;
        self.stats.flushes += 1;
        self.write_bytes(bytes);
        Some(())
    }

    /// Flushes all open buffers to root output in shutdown order.
    pub fn flush_all_buffers(&mut self) {
        while self.pop_buffer_flush().is_some() {}
        self.flush_root();
    }

    /// Flushes active buffer bytes to root output while keeping buffer levels open.
    pub fn flush_active_buffers_to_root(&mut self) {
        if self.stack.is_empty() {
            return;
        }
        if self.stack.iter().all(Vec::is_empty) {
            return;
        }
        for index in 0..self.stack.len() {
            let bytes = std::mem::take(&mut self.stack[index]);
            self.append_root(&bytes);
        }
        self.stats.flushes += 1;
    }

    /// Flushes only transport-facing root bytes. Active `ob_start()` levels
    /// are deliberately untouched.
    pub fn flush_root(&mut self) {
        if self.sink.is_none() || self.delivery_error.is_some() {
            return;
        }
        if !self.ensure_head_committed(None) {
            return;
        }
        if !self.bytes.is_empty() {
            let chunk = std::mem::take(&mut self.bytes);
            self.deliver_chunk(chunk);
        }
    }

    /// Completes root delivery and commits an empty response head if no body
    /// was produced.
    pub fn finish(&mut self) {
        if self.sink_finished {
            return;
        }
        while self.pop_buffer_flush().is_some() {}
        if self.sink.is_none() || self.delivery_error.is_some() {
            return;
        }
        if !self.ensure_head_committed(Some(self.stats.produced_bytes)) {
            return;
        }
        if !self.bytes.is_empty() {
            let chunk = std::mem::take(&mut self.bytes);
            self.deliver_chunk(chunk);
        }
        if let Some(sink) = self.sink.clone() {
            if let Err(error) = sink.0.finish() {
                self.fail(error);
            }
            self.sink_finished = true;
        }
    }

    #[must_use]
    pub fn delivery_error(&self) -> Option<&OutputDeliveryError> {
        self.delivery_error.as_ref()
    }

    #[must_use]
    pub fn is_streaming(&self) -> bool {
        self.sink.is_some()
    }

    #[must_use]
    pub fn http_response(&self) -> &RuntimeHttpResponseState {
        &self.response
    }

    pub fn http_response_mut(&mut self) -> &mut RuntimeHttpResponseState {
        &mut self.response
    }

    pub fn take_http_response(&mut self) -> RuntimeHttpResponseState {
        std::mem::take(&mut self.response)
    }

    pub fn set_http_response(&mut self, response: RuntimeHttpResponseState) {
        self.response = response;
    }

    fn append_root(&mut self, bytes: &[u8]) {
        self.stats.produced_bytes = self.stats.produced_bytes.saturating_add(bytes.len() as u64);
        if self.sink.is_none() {
            self.bytes.extend_from_slice(bytes);
            self.stats.max_root_pending_bytes = self.bytes.len();
            return;
        }
        if self.delivery_error.is_some() {
            return;
        }
        let mut remaining = bytes;
        while !remaining.is_empty() && self.delivery_error.is_none() {
            if self.bytes.is_empty() && remaining.len() >= OUTPUT_CHUNK_BYTES {
                let (chunk, rest) = remaining.split_at(OUTPUT_CHUNK_BYTES);
                self.deliver_chunk(chunk.to_vec());
                remaining = rest;
                continue;
            }
            let available = OUTPUT_CHUNK_BYTES - self.bytes.len();
            let take = available.min(remaining.len());
            self.bytes.extend_from_slice(&remaining[..take]);
            remaining = &remaining[take..];
            self.stats.max_root_pending_bytes =
                self.stats.max_root_pending_bytes.max(self.bytes.len());
            if self.bytes.len() == OUTPUT_CHUNK_BYTES {
                let chunk = std::mem::take(&mut self.bytes);
                self.deliver_chunk(chunk);
            }
        }
    }

    fn ensure_head_committed(&mut self, complete_length: Option<u64>) -> bool {
        if self.head_committed {
            return true;
        }
        let Some(sink) = self.sink.clone() else {
            return true;
        };
        match sink.0.commit(&self.response, complete_length) {
            Ok(()) => {
                self.head_committed = true;
                self.response.headers_sent = true;
                true
            }
            Err(error) => {
                self.fail(error);
                false
            }
        }
    }

    fn deliver_chunk(&mut self, chunk: Vec<u8>) {
        if chunk.is_empty() {
            return;
        }
        let commit_before_write = self
            .sink
            .as_ref()
            .is_none_or(|sink| sink.0.commit_before_write());
        if commit_before_write && !self.ensure_head_committed(None) {
            return;
        }
        let length = chunk.len() as u64;
        let Some(sink) = self.sink.clone() else {
            return;
        };
        match sink.0.write(chunk) {
            Ok(()) => {
                self.stats.delivered_bytes = self.stats.delivered_bytes.saturating_add(length);
                self.stats.delivered_chunks = self.stats.delivered_chunks.saturating_add(1);
            }
            Err(error) => self.fail(error),
        }
    }

    fn fail(&mut self, error: OutputDeliveryError) {
        if self.delivery_error.is_none() {
            self.delivery_error = Some(error);
            self.stats.delivery_failed = true;
        }
        self.bytes.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OUTPUT_CHUNK_BYTES, OutputBuffer, OutputDeliveryError, OutputSink, OutputSinkHandle,
    };
    use crate::context::RuntimeHttpResponseState;
    use std::sync::{Arc, Mutex, mpsc};

    #[derive(Clone)]
    struct RecordingSink {
        state: Arc<Mutex<(usize, Vec<Vec<u8>>)>>,
        fail_writes: bool,
    }

    impl OutputSink for RecordingSink {
        fn commit(
            &self,
            _response: &RuntimeHttpResponseState,
            _complete_length: Option<u64>,
        ) -> Result<(), OutputDeliveryError> {
            self.state.lock().expect("recording sink poisoned").0 += 1;
            Ok(())
        }

        fn write(&self, chunk: Vec<u8>) -> Result<(), OutputDeliveryError> {
            if self.fail_writes {
                return Err(OutputDeliveryError::new("receiver closed"));
            }
            self.state
                .lock()
                .expect("recording sink poisoned")
                .1
                .push(chunk);
            Ok(())
        }
    }

    #[test]
    fn nested_buffers_capture_clean_and_flush() {
        let mut output = OutputBuffer::new();
        output.write_test_str("root");
        output.start_buffer();
        output.write_test_str("a");
        output.start_buffer();
        output.write_test_str("b");

        assert_eq!(output.as_bytes(), b"root");
        assert_eq!(output.buffer_level(), 2);
        assert_eq!(output.current_buffer_bytes(), Some(&b"b"[..]));
        assert_eq!(output.pop_buffer_clean(), Some(b"b".to_vec()));
        assert_eq!(output.current_buffer_bytes(), Some(&b"a"[..]));
        assert_eq!(output.pop_buffer_flush(), Some(()));
        assert_eq!(output.as_bytes(), b"roota");
        assert_eq!(output.total_len(), 5);
        assert_eq!(output.stats().appends, 4);
        assert_eq!(output.stats().batch_writes, 0);
        assert_eq!(output.stats().batched_appends, 0);
        assert_eq!(output.stats().batch_bytes, 0);
        assert_eq!(output.stats().flushes, 1);
        assert_eq!(output.stats().fast_appends, 0);
        assert!(output.stats().slow_appends_by_reason.is_empty());
    }

    #[test]
    fn active_buffers_flush_to_root_without_closing_levels() {
        let mut output = OutputBuffer::new();
        output.write_test_str("root|");
        output.start_buffer();
        output.write_test_str("outer");
        output.start_buffer();
        output.write_test_str("inner");

        output.flush_active_buffers_to_root();

        assert_eq!(output.as_bytes(), b"root|outerinner");
        assert_eq!(output.total_len(), 15);
        assert_eq!(output.buffer_level(), 2);
        assert_eq!(output.current_buffer_bytes(), Some(&b""[..]));
        assert_eq!(output.stats().flushes, 1);

        output.write_test_str("tail");
        assert_eq!(output.pop_buffer_flush(), Some(()));
        assert_eq!(output.pop_buffer_flush(), Some(()));
        assert_eq!(output.as_bytes(), b"root|outerinnertail");
    }

    #[test]
    fn batch_write_reserves_and_counts_one_append() {
        let mut output = OutputBuffer::new();

        output.write_slices(&[b"a", b"", b"bc"]);

        assert_eq!(output.as_bytes(), b"abc");
        assert_eq!(output.stats().appends, 1);
        assert_eq!(output.stats().batch_writes, 1);
        assert_eq!(output.stats().batched_appends, 1);
        assert_eq!(output.stats().batch_bytes, 3);
        assert_eq!(output.stats().flushes, 0);
    }

    #[test]
    fn fast_and_slow_output_stats_are_stable() {
        let mut output = OutputBuffer::new();

        output.write_fast_bytes(b"a");
        output.write_fast_slices(&[b"b", b"c"]);
        output.record_slow_append_reason("object_to_string");
        output.record_slow_append_reason("object_to_string");

        assert_eq!(output.as_bytes(), b"abc");
        assert_eq!(output.stats().appends, 2);
        assert_eq!(output.stats().batch_writes, 1);
        assert_eq!(output.stats().batched_appends, 1);
        assert_eq!(output.stats().batch_bytes, 2);
        assert_eq!(output.stats().fast_appends, 2);
        assert_eq!(
            output
                .stats()
                .slow_appends_by_reason
                .get("object_to_string"),
            Some(&2)
        );
    }

    #[test]
    fn streaming_sink_chunks_large_writes_and_bounds_root_pending() {
        let state = Arc::new(Mutex::new((0, Vec::new())));
        let mut output = OutputBuffer::with_sink(OutputSinkHandle::new(RecordingSink {
            state: Arc::clone(&state),
            fail_writes: false,
        }));
        let bytes = vec![b'x'; OUTPUT_CHUNK_BYTES * 3 + 17];

        output.write_bytes(&bytes);
        output.finish();

        let state = state.lock().expect("recording sink poisoned");
        assert_eq!(state.0, 1);
        assert_eq!(state.1.iter().map(Vec::len).sum::<usize>(), bytes.len());
        assert!(
            state
                .1
                .iter()
                .all(|chunk| chunk.len() <= OUTPUT_CHUNK_BYTES)
        );
        assert!(output.stats().max_root_pending_bytes <= OUTPUT_CHUNK_BYTES);
        assert_eq!(output.stats().delivered_chunks, 4);
    }

    #[test]
    fn delivery_error_is_sticky_and_discards_followup_output() {
        let state = Arc::new(Mutex::new((0, Vec::new())));
        let mut output = OutputBuffer::with_sink(OutputSinkHandle::new(RecordingSink {
            state,
            fail_writes: true,
        }));

        output.write_bytes(vec![b'x'; OUTPUT_CHUNK_BYTES]);
        output.write_bytes(vec![b'y'; OUTPUT_CHUNK_BYTES * 4]);

        assert_eq!(
            output.delivery_error().map(|error| error.message()),
            Some("receiver closed")
        );
        assert!(output.as_bytes().is_empty());
        assert!(output.stats().delivery_failed);
        assert!(output.stats().max_root_pending_bytes <= OUTPUT_CHUNK_BYTES);
    }

    struct BlockingSink {
        sender: mpsc::SyncSender<Vec<u8>>,
    }

    impl OutputSink for BlockingSink {
        fn commit(
            &self,
            _response: &RuntimeHttpResponseState,
            _complete_length: Option<u64>,
        ) -> Result<(), OutputDeliveryError> {
            Ok(())
        }

        fn write(&self, chunk: Vec<u8>) -> Result<(), OutputDeliveryError> {
            self.sender
                .send(chunk)
                .map_err(|_| OutputDeliveryError::new("receiver closed"))
        }
    }

    #[test]
    fn bounded_sink_applies_synchronous_backpressure() {
        let (sender, receiver) = mpsc::sync_channel(1);
        let (finished_sender, finished_receiver) = mpsc::channel();
        let writer = std::thread::spawn(move || {
            let mut output =
                OutputBuffer::with_sink(OutputSinkHandle::new(BlockingSink { sender }));
            output.write_bytes(vec![b'x'; OUTPUT_CHUNK_BYTES * 3]);
            finished_sender.send(()).expect("report writer completion");
        });

        let first = receiver.recv().expect("first chunk");
        assert_eq!(first.len(), OUTPUT_CHUNK_BYTES);
        assert!(finished_receiver.try_recv().is_err());
        let second = receiver.recv().expect("second chunk");
        assert_eq!(second.len(), OUTPUT_CHUNK_BYTES);
        let third = receiver.recv().expect("third chunk");
        assert_eq!(third.len(), OUTPUT_CHUNK_BYTES);
        finished_receiver.recv().expect("writer completes");
        writer.join().expect("writer thread");
    }

    struct CountingHeadSink {
        state: Arc<Mutex<(Option<u64>, u64)>>,
    }

    impl OutputSink for CountingHeadSink {
        fn commit_before_write(&self) -> bool {
            false
        }

        fn commit(
            &self,
            _response: &RuntimeHttpResponseState,
            complete_length: Option<u64>,
        ) -> Result<(), OutputDeliveryError> {
            self.state.lock().expect("counting sink poisoned").0 = complete_length;
            Ok(())
        }

        fn write(&self, chunk: Vec<u8>) -> Result<(), OutputDeliveryError> {
            let mut state = self.state.lock().expect("counting sink poisoned");
            state.1 = state.1.saturating_add(chunk.len() as u64);
            Ok(())
        }
    }

    #[test]
    fn counting_sink_defers_head_and_retains_no_large_body() {
        let state = Arc::new(Mutex::new((None, 0)));
        let mut output = OutputBuffer::with_sink(OutputSinkHandle::new(CountingHeadSink {
            state: Arc::clone(&state),
        }));
        let length = OUTPUT_CHUNK_BYTES * 9 + 7;

        output.write_bytes(vec![b'x'; length]);
        assert_eq!(state.lock().expect("counting sink poisoned").0, None);
        output.finish();

        let state = state.lock().expect("counting sink poisoned");
        assert_eq!(state.0, Some(length as u64));
        assert_eq!(state.1, length as u64);
        assert!(output.as_bytes().is_empty());
        assert!(output.stats().max_root_pending_bytes <= OUTPUT_CHUNK_BYTES);
    }
}
