require "net/http"
require "uri"
require "fileutils"
require_relative "utils/banner"

MY_PROJECT = "Emberwing Miniapp"

RELEASES = [
  {
    name: "Linux x86_64",
    filename: "emberwing-bot-linux-x86_64",
    url: "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-linux-x86_64",
    chmod: true,
  },
  {
    name: "Linux aarch64",
    filename: "emberwing-bot-linux-aarch64",
    url: "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-linux-aarch64",
    chmod: true,
  },
  {
    name: "Linux armv7",
    filename: "emberwing-bot-linux-armv7",
    url: "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-linux-armv7",
    chmod: true,
  },
  {
    name: "Android aarch64",
    filename: "emberwing-bot-android-aarch64",
    url: "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-android-aarch64",
    chmod: true,
  },
  {
    name: "Android armv7",
    filename: "emberwing-bot-android-armv7",
    url: "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-android-armv7",
    chmod: true,
  },
  {
    name: "macOS aarch64",
    filename: "emberwing-bot-macos-aarch64",
    url: "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-macos-aarch64",
    chmod: true,
  },
  {
    name: "Windows x86_64",
    filename: "emberwing-bot-windows-x86_64.exe",
    url: "https://github.com/Yuurisan-N1/Emberwing-Miniapp/releases/latest/download/emberwing-bot-windows-x86_64.exe",
    chmod: false,
  },
].freeze

G   = "\e[1m\e[32m"
Y   = "\e[1m\e[33m"
R   = "\e[1m\e[31m"
RST = "\e[0m"

def green(text)  = "#{G}#{text}#{RST}"
def yellow(text) = "#{Y}#{text}#{RST}"
def red(text)    = "#{R}#{text}#{RST}"

trap("INT") do
  $stdout.write("\n")
  $stdout.write("#{R}Script stopped by user#{RST}\n")
  $stdout.flush
  exit(0)
end

def download_file(url, filename, chmod)
  uri = URI.parse(url)

  Net::HTTP.start(uri.host, uri.port, use_ssl: uri.scheme == "https") do |http|
    request = Net::HTTP::Get.new(uri.request_uri)

    http.request(request) do |resp|
      # Follow redirect manually if needed
      if resp.is_a?(Net::HTTPRedirection)
        return download_file(resp["location"], filename, chmod)
      end

      total      = resp["content-length"]&.to_i || 0
      downloaded = 0
      chunk_size = 8192

      File.open(filename, "wb") do |f|
        resp.read_body do |chunk|
          next if chunk.empty?

          f.write(chunk)
          downloaded += chunk.bytesize

          if total > 0
            percent  = downloaded.to_f / total * 100
            filled   = (percent / 2).to_i
            bar      = " " * filled + "." * (50 - filled)
            mb_done  = downloaded.to_f / 1024 / 1024
            mb_total = total.to_f / 1024 / 1024
            line     = "\rDownloading #{bar} #{format("%.1f", percent)}% #{format("%.2f", mb_done)} MB of #{format("%.2f", mb_total)} MB   "
            $stdout.write("#{G}#{line}#{RST}")
          else
            mb_done = downloaded.to_f / 1024 / 1024
            line    = "\rDownloading #{format("%.2f", mb_done)} MB   "
            $stdout.write("#{Y}#{line}#{RST}")
          end
          $stdout.flush
        end
      end
    end
  end

  $stdout.write("\r" + " " * 80 + "\r")
  $stdout.flush

  File.chmod(0o755, filename) if chmod
end

def main
  show_banner(MY_PROJECT)

  puts yellow("Select the binary you want to download:")
  puts

  RELEASES.each_with_index do |release, i|
    puts green("#{i + 1}. #{release[:name]}")
  end
  puts

  choice = nil
  loop do
    begin
      $stdout.write("#{Y}Enter number: #{RST}")
      $stdout.flush
      raw    = $stdin.gets&.strip
      choice = Integer(raw)
      break if choice >= 1 && choice <= RELEASES.size

      puts red("Please enter a number between 1 and #{RELEASES.size}")
    rescue ArgumentError, TypeError
      puts red("Invalid input please enter a valid number")
    end
  end

  selected = RELEASES[choice - 1]
  puts
  puts yellow("Starting download #{selected[:name]}")

  begin
    download_file(selected[:url], selected[:filename], selected[:chmod])
    puts green("Download complete saved as #{selected[:filename]}")
    puts green("File permission set to executable") if selected[:chmod]
  rescue => e
    puts red("Download failed")
    exit(1)
  end
end

main
