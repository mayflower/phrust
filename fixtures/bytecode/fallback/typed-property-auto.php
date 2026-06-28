<?php
class BytecodeTypedPropertyTarget {
    public int $value = 7;
}

echo (new BytecodeTypedPropertyTarget())->value, "\n";
