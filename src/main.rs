use std::io::Read;

use ssh2::Session;

fn main() {
    let session = create_session();
    remote_ls_output(&session);

    // via scp
    let file = b"1234567890";
    upload_file_via_scp(&session, file);
    remote_ls_output(&session);

    // lets try openssh sftp client crate
    let runtime = tokio::runtime::Runtime::new().unwrap();
    runtime.block_on(async {
        upload_file_via_sftp(&session, file).await;

        // sync so be careful here
        remote_ls_output(&session);
    })
}

fn remote_ls_output(session: &Session) {
    let mut channel = session.channel_session().expect("Couldnt create a channel");
    channel.exec("ls").unwrap();
    let mut output = String::new();
    channel
        .read_to_string(&mut output)
        .expect("Unable to read string");
    println!("{}", output);
}

fn create_session() -> Session {
    let username =
        std::env::var("SSH_USERNAME").expect("Please pass ssh username via environment variable");
    let password =
        std::env::var("SSH_PASSWORD").expect("Please pass ssh password via environment variable");

    let stream = std::net::TcpStream::connect("127.0.0.1:22").unwrap();

    let mut session = Session::new().expect("unable to create ssh session");
    session.set_tcp_stream(stream);
    session
        .handshake()
        .expect("Unable to make handshake with ssh server");

    session
        .userauth_password(&username, &password)
        .expect("Invalid ssh username or password");

    assert!(session.authenticated());
    session
}

fn upload_file_via_scp(session: &Session, file: &[u8]) {
    use std::io::Write;

    let mut remote_file = session
        .scp_send(std::path::Path::new("delete_it"), 0o644, 10, None)
        .unwrap();
    remote_file.write(file).unwrap();
    // Close the channel and wait for the whole content to be tranferred
    remote_file.send_eof().unwrap();
    remote_file.wait_eof().unwrap();
    remote_file.close().unwrap();
    remote_file.wait_close().unwrap();
}

async fn upload_file_via_sftp(
    session: &Session,
    file_to_upload: &[u8],
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    use openssh_sftp_client::highlevel::Sftp;

    let ssftp = session.sftp().unwrap();
    let mut file = ssftp.open(std::path::Path::new("dummy.txt")).unwrap();

    // ERR
    let (reader, writer) = std::io::split(file);
    let sftp = Sftp::new(writer, reader, Default::default()).await?;
    let mut fs = sftp.fs();

    fs.write(std::path::Path::new("delete_it_3"), file_to_upload);
    Ok(())
}
