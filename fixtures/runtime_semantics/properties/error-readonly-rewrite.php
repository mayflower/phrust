<?php
// runtime-semantics: expect=fail
class ReadonlyRewrite {
    public readonly int $value;
    public function set(int $value) {
        $this->value = $value;
    }
}
$item = new ReadonlyRewrite();
$item->set(1);
$item->set(2);
