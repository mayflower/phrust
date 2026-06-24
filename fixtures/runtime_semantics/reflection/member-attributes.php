<?php
// runtime-semantics: category=reflection expect=pass
class AttributeMemberTarget {
    #[PropertyMarker("property")]
    public string $name = "ok";

    #[ConstMarker("constant")]
    public const LABEL = "label";

    #[MethodMarker("method")]
    public function run(#[ParamMarker("param")] string $value): string {
        return $value;
    }
}

$class = new ReflectionClass(AttributeMemberTarget::class);

$methodAttributes = $class->getMethod("run")->getAttributes();
echo $methodAttributes[0]->getName(), ":", $methodAttributes[0]->getArguments()[0], "\n";

$parameters = $class->getMethod("run")->getParameters();
$parameterAttributes = $parameters[0]->getAttributes();
echo $parameterAttributes[0]->getName(), ":", $parameterAttributes[0]->getArguments()[0], "\n";

$propertyAttributes = $class->getProperty("name")->getAttributes();
echo $propertyAttributes[0]->getName(), ":", $propertyAttributes[0]->getArguments()[0], "\n";

$constantAttributes = $class->getReflectionConstant("LABEL")->getAttributes();
echo $constantAttributes[0]->getName(), ":", $constantAttributes[0]->getArguments()[0], "\n";
