pub trait Input {
    type InputType;

    fn from_input(input: Self::InputType) -> Self;
    fn merge_input(self, input: Self::InputType) -> Self;
}
