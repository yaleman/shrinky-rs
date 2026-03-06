use shrinky_rs::SsimQuality;

#[test]
fn test_ssim_quality_boundaries_and_meaning() {
    let cases = [
        (1.0, SsimQuality::Identical, "identical images"),
        (1.0001, SsimQuality::Identical, "identical images"),
        (0.95, SsimQuality::ExtremelySimilar, "extremely similar"),
        (0.9, SsimQuality::ExtremelySimilar, "extremely similar"),
        (0.89, SsimQuality::SmallVisibleDifferences, "small visible differences"),
        (0.8, SsimQuality::SmallVisibleDifferences, "small visible differences"),
        (0.79, SsimQuality::SmallVisibleDifferences, "small visible differences"),
        (0.7, SsimQuality::SmallVisibleDifferences, "small visible differences"),
        (0.0, SsimQuality::NoticeableDegradation, "noticeable degradation"),
    ];

    for (ssim, expected_quality, expected_meaning) in cases {
        let mapped = SsimQuality::from_ssim(ssim).expect("Expected SSIM quality bucket");
        assert_eq!(mapped, expected_quality, "Unexpected bucket for SSIM {ssim}");
        assert_eq!(
            mapped.meaning(),
            expected_meaning,
            "Unexpected meaning for SSIM {ssim}"
        );
    }
}

#[test]
fn test_ssim_quality_rejects_nan() {
    assert!(SsimQuality::from_ssim(f64::NAN).is_none());
}
