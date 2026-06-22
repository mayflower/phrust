//! Stack frame and register storage for the first VM core.

use php_ir::ids::{FunctionId, LocalId, RegId};
use php_runtime::{ReferenceCell, Slot, TempValue, Value};

/// Register storage with checked accessors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RegisterFile {
    registers: Vec<TempValue>,
}

/// Local storage with checked accessors.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalFile {
    locals: Vec<Slot>,
}

impl LocalFile {
    /// Creates local storage filled with `Uninitialized`.
    #[must_use]
    pub fn new(count: u32) -> Self {
        Self {
            locals: vec![Slot::uninitialized(); count as usize],
        }
    }

    /// Reads a local without panicking.
    #[must_use]
    pub fn get(&self, id: LocalId) -> Option<Value> {
        self.locals.get(id.index()).map(Slot::read)
    }

    /// Iterates over local slots in stable slot order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (usize, &Slot)> {
        self.locals.iter().enumerate()
    }

    /// Reads a local slot mutably without panicking.
    pub fn get_slot_mut(&mut self, id: LocalId) -> Option<&mut Slot> {
        self.locals.get_mut(id.index())
    }

    /// Writes a local without panicking.
    pub fn set(&mut self, id: LocalId, value: Value) -> Result<(), String> {
        let Some(slot) = self.locals.get_mut(id.index()) else {
            return Err(format!("invalid local local:{}", id.raw()));
        };
        slot.write(value);
        Ok(())
    }

    /// Unsets a local name without writing through a referenced alias cell.
    pub fn unset(&mut self, id: LocalId) -> Result<(), String> {
        let Some(slot) = self.locals.get_mut(id.index()) else {
            return Err(format!("invalid local local:{}", id.raw()));
        };
        *slot = Slot::uninitialized();
        Ok(())
    }

    /// Binds `target` to the same reference cell as `source`.
    pub fn bind_reference(&mut self, target: LocalId, source: LocalId) -> Result<(), String> {
        if target.index() >= self.locals.len() {
            return Err(format!("invalid local local:{}", target.raw()));
        }
        let Some(source_slot) = self.locals.get_mut(source.index()) else {
            return Err(format!("invalid local local:{}", source.raw()));
        };
        let cell: ReferenceCell = source_slot.ensure_reference_cell();
        let target_slot = self
            .locals
            .get_mut(target.index())
            .expect("target bounds checked");
        target_slot.bind_reference(cell);
        Ok(())
    }

    /// Converts a local to a reference cell and returns that shared cell.
    pub fn ensure_reference_cell(&mut self, id: LocalId) -> Result<ReferenceCell, String> {
        let Some(slot) = self.locals.get_mut(id.index()) else {
            return Err(format!("invalid local local:{}", id.raw()));
        };
        Ok(slot.ensure_reference_cell())
    }

    /// Binds a local to an existing reference cell.
    pub fn bind_reference_cell(&mut self, id: LocalId, cell: ReferenceCell) -> Result<(), String> {
        let Some(slot) = self.locals.get_mut(id.index()) else {
            return Err(format!("invalid local local:{}", id.raw()));
        };
        slot.bind_reference(cell);
        Ok(())
    }
}

impl RegisterFile {
    /// Creates a register file filled with `Uninitialized`.
    #[must_use]
    pub fn new(count: u32) -> Self {
        Self {
            registers: vec![TempValue::uninitialized(); count as usize],
        }
    }

    /// Returns the number of registers.
    #[must_use]
    pub fn len(&self) -> usize {
        self.registers.len()
    }

    /// Returns true when no registers are allocated.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.registers.is_empty()
    }

    /// Reads a register without panicking.
    #[must_use]
    pub fn get(&self, id: RegId) -> Option<&Value> {
        self.registers.get(id.index()).map(TempValue::value)
    }

    /// Iterates over registers in stable register order.
    pub fn iter(&self) -> impl ExactSizeIterator<Item = (usize, &Value)> {
        self.registers
            .iter()
            .enumerate()
            .map(|(index, value)| (index, value.value()))
    }

    /// Reads a register mutably without panicking.
    pub fn get_mut(&mut self, id: RegId) -> Option<&mut Value> {
        self.registers.get_mut(id.index()).map(TempValue::value_mut)
    }

    /// Writes a register without panicking.
    pub fn set(&mut self, id: RegId, value: Value) -> Result<(), String> {
        let Some(slot) = self.registers.get_mut(id.index()) else {
            return Err(format!("invalid register r{}", id.raw()));
        };
        slot.set(value);
        Ok(())
    }
}

/// One executing frame.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Frame {
    /// Function being executed.
    pub function: FunctionId,
    /// Class scope used for `self::`, visibility, and private member lookup.
    pub scope_class: Option<String>,
    /// Late-static-binding called class used for `static::`.
    pub called_class: Option<String>,
    /// Class that declares the selected method body.
    pub declaring_class: Option<String>,
    /// Registers for the function.
    pub registers: RegisterFile,
    /// PHP local variable slots for the function.
    pub locals: LocalFile,
}

impl Frame {
    /// Creates a frame for a function.
    #[must_use]
    pub fn new(function: FunctionId, register_count: u32, local_count: u32) -> Self {
        Self {
            function,
            scope_class: None,
            called_class: None,
            declaring_class: None,
            registers: RegisterFile::new(register_count),
            locals: LocalFile::new(local_count),
        }
    }

    /// Creates a frame for a class method with explicit class metadata.
    #[must_use]
    pub fn new_with_class_context(
        function: FunctionId,
        register_count: u32,
        local_count: u32,
        scope_class: Option<String>,
        called_class: Option<String>,
        declaring_class: Option<String>,
    ) -> Self {
        Self {
            function,
            scope_class,
            called_class,
            declaring_class,
            registers: RegisterFile::new(register_count),
            locals: LocalFile::new(local_count),
        }
    }
}

/// Minimal call stack container.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct CallStack {
    frames: Vec<Frame>,
}

impl CallStack {
    /// Creates an empty call stack.
    #[must_use]
    pub const fn new() -> Self {
        Self { frames: Vec::new() }
    }

    /// Pushes a frame.
    pub fn push(&mut self, frame: Frame) {
        self.frames.push(frame);
    }

    /// Pops a frame.
    pub fn pop(&mut self) -> Option<Frame> {
        self.frames.pop()
    }

    /// Returns the top frame.
    #[must_use]
    pub fn current(&self) -> Option<&Frame> {
        self.frames.last()
    }

    /// Returns the top frame mutably.
    pub fn current_mut(&mut self) -> Option<&mut Frame> {
        self.frames.last_mut()
    }

    /// Returns frames from entry to current frame.
    #[must_use]
    pub fn frames(&self) -> &[Frame] {
        &self.frames
    }

    /// Returns the number of frames.
    #[must_use]
    pub fn len(&self) -> usize {
        self.frames.len()
    }

    /// Returns true when no frames are active.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.frames.is_empty()
    }
}
