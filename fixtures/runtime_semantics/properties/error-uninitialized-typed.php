<?php
// runtime-semantics: expect=fail
class UninitializedTypedProperty {
    public int $count;
}
echo (new UninitializedTypedProperty())->count;
