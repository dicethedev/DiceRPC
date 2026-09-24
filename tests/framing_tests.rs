use dice_rpc::transport::framing::FrameCodec;
use tokio::io::BufReader;

#[tokio::test]
async fn test_frame_codec() {
    let data = b"Hello, World!";
    let mut buffer = Vec::new();

    FrameCodec::write_frame(&mut buffer, data).await.unwrap();

    let mut reader = BufReader::new(&buffer[..]);
    let result = FrameCodec::read_frame(&mut reader).await.unwrap();

    assert_eq!(result, data);
}

#[tokio::test]
async fn test_multiple_frames() {
    let messages: Vec<&[u8]> = vec![b"first", b"second message", b"third"];
    let mut buffer = Vec::new();

    for msg in &messages {
        FrameCodec::write_frame(&mut buffer, msg).await.unwrap();
    }

    let mut reader = BufReader::new(&buffer[..]);

    for expected in &messages {
        let result = FrameCodec::read_frame(&mut reader).await.unwrap();
        assert_eq!(result, *expected);
    }
}

#[tokio::test]
async fn oversized_frame_is_rejected_before_payload_allocation() {
    let mut bytes = Vec::from((1024_u32).to_be_bytes());
    bytes.extend_from_slice(b"ignored");
    let mut reader = BufReader::new(bytes.as_slice());
    let error = FrameCodec::read_frame_with_limit(&mut reader, 64)
        .await
        .unwrap_err();
    assert!(error.to_string().contains("Frame too large"));
}
