<?php
function include_declared_symbol_function() {
    return "function";
}

class IncludeDeclaredSymbolClass {
    public const VALUE = "class";

    public function value() {
        return self::VALUE;
    }
}

const INCLUDE_DECLARED_SYMBOL_CONST = "const";
