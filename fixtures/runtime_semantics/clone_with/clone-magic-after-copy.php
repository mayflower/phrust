<?php
// runtime-semantics: category=clone_with expect=pass
class CloneMagicAfterCopy {
    public int $value = 1;

    public function __clone(): void {
        $this->value = $this->value + 1;
    }
}

$original = new CloneMagicAfterCopy();
$copy = clone $original;
echo $original->value . "|" . $copy->value;
