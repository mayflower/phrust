<?php
namespace RuntimeNamespaceFixture;

const INCLUDED_CONST = "const";

function included_function() {
    return "function";
}

class IncludedClass {
    public function value() {
        return "class";
    }
}
