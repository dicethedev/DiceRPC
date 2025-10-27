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
