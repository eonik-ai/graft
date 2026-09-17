# SPDX-License-Identifier: Apache-2.0
# Homebrew formula. Install from this repo:
#
#   brew tap eonik-ai/graft https://github.com/eonik-ai/graft
#   brew install graft
#
# ffmpeg is a runtime dependency (graft shells out; it does not link x264).
class Graft < Formula
  desc "Change the hook. Keep the body."
  homepage "https://github.com/eonik-ai/graft"
  license "Apache-2.0"
  head "https://github.com/eonik-ai/graft.git", branch: "main"

  depends_on "rust" => :build
  depends_on "ffmpeg"

  def install
    system "cargo", "install", *std_cargo_args(path: "crates/graft")
  end

  def caveats
    <<~EOS
      graft shells out to ffmpeg and ffprobe. They are already in this keg's
      dependency tree. Override with $FFMPEG / $FFPROBE if you need a custom build.
    EOS
  end

  test do
    assert_match "graft", shell_output("#{bin}/graft --version")
  end
end
