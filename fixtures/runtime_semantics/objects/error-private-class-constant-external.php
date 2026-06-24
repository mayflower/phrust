<?php
// runtime-semantics: expect=fail
class PrivateClassConstant {
    private const SECRET = 'hidden';
}

echo PrivateClassConstant::SECRET, "\n";
