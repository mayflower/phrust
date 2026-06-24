<?php
namespace Stdlib\Corpus;

class ContainerService
{
    public function label($name)
    {
        return 'service:' . \strtoupper($name);
    }
}
