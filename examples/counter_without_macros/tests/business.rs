mod count {
    use counter_without_macros::Count;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(-2, 1)]
    #[case(-1, 1)]
    #[case(0, 1)]
    #[case(1, 1)]
    #[case(2, 1)]
    #[case(3, 2)]
    #[case(4, 3)]
    fn should_increase_count(#[case] initial_value: i32, #[case] step: u32) {
        let mut count = Count::new(initial_value, step);
        count.increase();
        assert_eq!(count.value(), initial_value + step as i32);
    }

    #[rstest]
    #[case(-2, 1)]
    #[case(-1, 1)]
    #[case(0, 1)]
    #[case(1, 1)]
    #[case(2, 1)]
    #[case(3, 2)]
    #[case(4, 3)]
    #[trace]
    fn should_decrease_count(#[case] initial_value: i32, #[case] step: u32) {
        let mut count = Count::new(initial_value, step);
        count.decrease();
        assert_eq!(count.value(), initial_value - step as i32);
    }

    #[rstest]
    #[case(-2, 1)]
    #[case(-1, 1)]
    #[case(0, 1)]
    #[case(1, 1)]
    #[case(2, 1)]
    #[case(3, 2)]
    #[case(4, 3)]
    #[trace]
    fn should_clear_count(#[case] initial_value: i32, #[case] step: u32) {
        let mut count = Count::new(initial_value, step);
        count.clear();
        assert_eq!(count.value(), 0);
    }
}
