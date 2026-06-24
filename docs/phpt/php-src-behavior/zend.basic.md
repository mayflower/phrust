# Zend Basic Behavior Notes

This note covers the selected Zend basic PHPT batch for scalar literal
execution, output statements, `var_dump` output, statement sequencing, and
basic top-level control flow.

## Source Notes

- `Zend/tests/numeric_literal_separator/numeric_literal_separator_001.phpt`
  is the selected upstream PHPT for numeric literal separators. Reference PHP
  8.5.7 accepts separators in decimal integers, floats, scientific notation,
  hex integers, binary integers, and legacy octal integers. Equivalent
  literals compare strictly equal and `var_dump` prints `bool(true)`.
- `Zend/zend_language_parser.y` maps numeric literals through `T_LNUMBER` and
  `T_DNUMBER`, so the separators are already resolved before execution.
- `Zend/zend_compile.c` lowers echo AST nodes to `ZEND_ECHO`, and
  `Zend/zend_vm_def.h`/`Zend/zend_vm_execute.h` contain the corresponding VM
  handlers for output.
- Source-symbol lookup for `var_dump` resolves to
  `ext/standard/var.c:240` (`PHP_FUNCTION(var_dump)`). The implementation
  iterates over variadic arguments and delegates scalar formatting to
  `php_var_dump`.

## Implementation Notes

- Phrust already lowered scalar `echo`, `print`, strict comparison, and basic
  `var_dump` enough for the selected test.
- The missing behavior was numeric literal lowering for prefixed integer
  literals with separators. `php_ir` now strips separators and parses hex,
  binary, decimal, and legacy octal integer literal spellings before VM
  execution.
- Float detection remains decimal-only in this lowering path so `0xCAFE_F00D`
  is not misclassified as an exponent-form float because its hex digits contain
  `E`.
