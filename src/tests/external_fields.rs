mod timestamp_tests {
    use crate::archive_common::{
        ArchiveDescriptor, ArchiveFileEntry, ExtraField, ExtraFieldExtendedTimestamp,
    };

    const TEST_DATA: Option<i32> = Some(1582248020);
    #[test]
    fn test_flags() {
        let extrafield = ExtraFieldExtendedTimestamp::new(TEST_DATA, None, None);

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT
        );

        let extrafield = ExtraFieldExtendedTimestamp::new(TEST_DATA, TEST_DATA, TEST_DATA);

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT
                | ExtraFieldExtendedTimestamp::CREATE_TIME_BIT
                | ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT
        );

        let extrafield = ExtraFieldExtendedTimestamp::new(TEST_DATA, None, TEST_DATA);

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT
                | ExtraFieldExtendedTimestamp::CREATE_TIME_BIT
        );

        let extrafield = ExtraFieldExtendedTimestamp::new(Some(1582248020), TEST_DATA, TEST_DATA);

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::MODIFY_TIME_BIT
                | ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT
                | ExtraFieldExtendedTimestamp::CREATE_TIME_BIT
        );

        let extrafield = ExtraFieldExtendedTimestamp::new(None, TEST_DATA, None);

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT
        );

        let extrafield = ExtraFieldExtendedTimestamp::new(None, TEST_DATA, TEST_DATA);

        assert_eq!(
            extrafield.flags,
            ExtraFieldExtendedTimestamp::ACCESS_TIME_BIT
                | ExtraFieldExtendedTimestamp::CREATE_TIME_BIT
        );
    }

    #[test]
    fn test_write() {
        let extrafield = ExtraFieldExtendedTimestamp::new(TEST_DATA, None, None);

        let size = extrafield.file_header_extra_field_data_size();

        assert_eq!(size, 5);

        let extrafield = ExtraFieldExtendedTimestamp::new(None, None, None);

        let size = extrafield.file_header_extra_field_data_size();
        assert_eq!(size, 1);

        let mut archive_descriptor = ArchiveDescriptor::new(100);

        let archive_file_entry = ArchiveFileEntry::default();

        extrafield.central_header_extra_write_data(&mut archive_descriptor, &archive_file_entry);

        assert!(archive_descriptor.is_empty());

        extrafield.local_header_write_data(&mut archive_descriptor, &archive_file_entry);

        assert!(archive_descriptor.is_empty());

        let extrafield = ExtraFieldExtendedTimestamp::new(TEST_DATA, TEST_DATA, TEST_DATA);

        extrafield.central_header_extra_write_data(&mut archive_descriptor, &archive_file_entry);

        assert!(!archive_descriptor.is_empty());
        assert_eq!(
            archive_descriptor.len(),
            extrafield.central_header_extra_field_size(&archive_file_entry) as usize
        );

        archive_descriptor.clear();
        extrafield.local_header_write_data(&mut archive_descriptor, &archive_file_entry);

        assert!(!archive_descriptor.is_empty());
        assert_eq!(
            archive_descriptor.len(),
            extrafield.local_header_extra_field_size(&archive_file_entry) as usize
        );
    }
}
