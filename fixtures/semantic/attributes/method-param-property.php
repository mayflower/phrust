<?php

class AttributeMemberTarget
{
    #[PropertyAttribute("property")]
    public string $name = "value";

    #[MethodAttribute]
    public function run(
        #[ParameterAttribute(["name" => "value"])]
        string $name,
    ): void {}
}
