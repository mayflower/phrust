<?php
// phase5-runtime: expect=fail
class UninitializedTypedProperty {
    public int $count;
}
echo (new UninitializedTypedProperty())->count;
