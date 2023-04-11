mod timestamp_tests {
    use crate::archive_common::ExtraFieldExtendedTimestamp;

    #[test]
    fn test_flags() {
        let extrafield = ExtraFieldExtendedTimestamp::new(Some(1582248020), None, None);

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT
        );

        let extrafield =
            ExtraFieldExtendedTimestamp::new(Some(1582248020), Some(1582248020), Some(1582248020));

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT
                | ExtraFieldExtendedTimestamp::CREATE_TIME_BIT
                | ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT
        );

        let extrafield = ExtraFieldExtendedTimestamp::new(Some(1582248020), None, Some(1582248020));

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT
                | ExtraFieldExtendedTimestamp::CREATE_TIME_BIT
        );

        let extrafield =
            ExtraFieldExtendedTimestamp::new(Some(1582248020), Some(1582248020), Some(1582248020));

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT
                | ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT
                | ExtraFieldExtendedTimestamp::CREATE_TIME_BIT
        );

        let extrafield = ExtraFieldExtendedTimestamp::new(None, Some(1582248020), None);

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT
        );

        let extrafield = ExtraFieldExtendedTimestamp::new(None, Some(1582248020), Some(1582248020));

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT
                | ExtraFieldExtendedTimestamp::CREATE_TIME_BIT
        );
    }
}
