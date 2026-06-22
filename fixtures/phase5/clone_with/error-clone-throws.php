<?php
// phase5-runtime: category=clone_with expect=fail
class CloneThrows {
    public function __clone(): void {
        throw new Exception("clone failed");
    }
}

clone new CloneThrows();
