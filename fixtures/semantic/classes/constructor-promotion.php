<?php

class PromotedSignature
{
    public function __construct(
        public readonly string $id,
        protected int $count = 0,
        public private(set) ?string $name = null,
    ) {
    }
}
