<?php
// phase5-runtime: category=magic expect=fail
class MagicToStringThrows {
    public function __toString(): string {
        throw new Exception("boom");
    }
}

echo new MagicToStringThrows();
