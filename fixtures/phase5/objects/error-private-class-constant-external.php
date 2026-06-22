<?php
// phase5-runtime: expect=fail
class PrivateClassConstant {
    private const SECRET = 'hidden';
}

echo PrivateClassConstant::SECRET, "\n";
