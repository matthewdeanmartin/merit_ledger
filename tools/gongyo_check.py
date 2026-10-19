"""Check the gongyo romanization in `meritledger_ui/public/texts/` against CBETA.

The recited portions of Lotus Sutra chapters 2 and 16 are kept here as
(Chinese phrase, romaji) pairs, one reading per Chinese character: in the
romaji, spaces separate printed words, and `·` or `-` separate the syllables
of one word. The check:

- the joined Chinese equals CBETA's text of the passage (punctuation removed);
- every phrase has exactly one reading per character;
- the text files say each phrase as `say()` prints it.

So no phrase can be dropped, doubled or misaligned. Whether each reading is the
one your gongyo book prints is for a person to review.

    uv run --no-project python tools/gongyo_check.py   # needs the network
"""

import html
import json
import re
import ssl
import sys
import urllib.parse
import urllib.request
from pathlib import Path

API = "https://cbdata.dila.edu.tw/stable/lines"
TEXTS = Path(__file__).resolve().parent.parent / "meritledger_ui" / "public" / "texts"
# CBETA's certificate lacks a Subject Key Identifier, which Python 3.13's
# strict X.509 mode rejects. The chain and hostname are still verified.
TLS = ssl.create_default_context()
TLS.verify_flags &= ~ssl.VERIFY_X509_STRICT
PUNCT = re.compile(r"[\s、，。！？：；「」『』]")

HOBEN = [
    ("爾時世尊", "ni·ji se·son"),
    ("從三昧", "ju san·mai"),
    ("安詳而起", "an·jo ni ki"),
    ("告舍利弗", "go sha·ri-hotsu"),
    ("諸佛智慧", "sho but·chi e"),
    ("甚深無量", "jin·jin mu·ryo"),
    ("其智慧門", "go chi·e mon"),
    ("難解難入", "nan·ge nan·nyu"),
    ("一切聲聞", "is·sai sho·mon"),
    ("辟支佛", "hyaku-shi-butsu"),
    ("所不能知", "sho fu no chi"),
    ("所以者何", "sho·i sha ga"),
    ("佛曾親近", "butsu zo shin·gon"),
    ("百千萬億", "hyaku sen man noku"),
    ("無數諸佛", "mu·shu shi butsu"),
    ("盡行諸佛", "jin gyo shi butsu"),
    ("無量道法", "mu·ryo do·ho"),
    ("勇猛精進", "yu·myo sho·jin"),
    ("名稱普聞", "myo·sho fu·mon"),
    ("成就甚深", "jo·ju jin·jin"),
    ("未曾有法", "mi-zo-u ho"),
    ("隨宜所說", "zui·gi sho setsu"),
    ("意趣難解", "i·shu nan·ge"),
    ("舍利弗", "sha·ri-hotsu"),
    ("吾從成佛已來", "go ju jo·butsu i·rai"),
    ("種種因緣", "shu·ju in·nen"),
    ("種種譬喻", "shu·ju hi·yu"),
    ("廣演言教", "ko en gon·kyo"),
    ("無數方便", "mu·shu ho·ben"),
    ("引導眾生", "in·do shu·jo"),
    ("令離諸著", "ryo ri sho jaku"),
    ("所以者何", "sho·i sha ga"),
    ("如來方便", "nyo·rai ho·ben"),
    ("知見波羅蜜", "chi·ken ha·ra·mitsu"),
    ("皆已具足", "kai i gu·soku"),
    ("舍利弗", "sha·ri-hotsu"),
    ("如來知見", "nyo·rai chi·ken"),
    ("廣大深遠", "ko·dai jin·non"),
    ("無量無礙", "mu·ryo mu·ge"),
    ("力", "riki"),
    ("無所畏", "mu·sho·i"),
    ("禪定", "zen·jo"),
    ("解脫三昧", "ge·das san·mai"),
    ("深入無際", "jin·nyu mu·sai"),
    ("成就一切", "jo·ju is·sai"),
    ("未曾有法", "mi-zo-u ho"),
    ("舍利弗", "sha·ri-hotsu"),
    ("如來能種種分別", "nyo·rai no shu·ju fun·betsu"),
    ("巧說諸法", "gyo ses·sho ho"),
    ("言辭柔軟", "gon·ji nyu·nan"),
    ("悅可眾心", "ek·ka shu·shin"),
    ("舍利弗", "sha·ri-hotsu"),
    ("取要言之", "shu yo gon shi"),
    ("無量無邊", "mu·ryo mu·hen"),
    ("未曾有法", "mi-zo-u ho"),
    ("佛悉成就", "bus·shitsu jo·ju"),
    ("止舍利弗", "shi sha·ri-hotsu"),
    ("不須復說", "fu shu bu setsu"),
    ("所以者何", "sho·i sha ga"),
    ("佛所成就", "bus·sho jo·ju"),
    ("第一希有", "dai·ichi ke·u"),
    ("難解之法", "nan·ge shi ho"),
    ("唯佛與佛", "yui butsu yo butsu"),
    ("乃能究盡", "nai no ku·jin"),
    ("諸法實相", "sho·ho jis·so"),
    ("所謂諸法", "sho·i sho·ho"),
    ("如是相", "nyo·ze so"),
    ("如是性", "nyo·ze sho"),
    ("如是體", "nyo·ze tai"),
    ("如是力", "nyo·ze riki"),
    ("如是作", "nyo·ze sa"),
    ("如是因", "nyo·ze in"),
    ("如是緣", "nyo·ze en"),
    ("如是果", "nyo·ze ka"),
    ("如是報", "nyo·ze ho"),
    ("如是本末究竟等", "nyo·ze hon·mak ku·kyo to"),
]

JIGAGE = """\
自我得佛來|ji ga toku butsu rai
所經諸劫數|sho kyo sho kos·shu
無量百千萬|mu·ryo hyaku sen man
億載阿僧祇|oku sai a·so·gi
常說法教化|jo sep·po kyo·ke
無數億眾生|mu·shu oku shu·jo
令入於佛道|ryo nyu o butsu·do
爾來無量劫|ni·rai mu·ryo ko
為度眾生故|i do shu·jo ko
方便現涅槃|ho·ben gen ne·han
而實不滅度|ni jitsu fu metsu·do
常住此說法|jo ju shi sep·po
我常住於此|ga jo ju o shi
以諸神通力|i sho jin·zu riki
令顛倒眾生|ryo ten·do shu·jo
雖近而不見|sui gon ni fu ken
眾見我滅度|shu ken ga metsu·do
廣供養舍利|ko ku·yo sha·ri
咸皆懷戀慕|gen kai e ren·bo
而生渴仰心|ni sho katsu·go shin
眾生既信伏|shu·jo ki shin·buku
質直意柔軟|shichi·jiki i nyu·nan
一心欲見佛|is·shin yoku ken butsu
不自惜身命|fu ji shaku shin·myo
時我及眾僧|ji ga gyu shu·so
俱出靈鷲山|ku shutsu ryo·ju sen
我時語眾生|ga ji go shu·jo
常在此不滅|jo zai shi fu metsu
以方便力故|i ho·ben rik·ko
現有滅不滅|gen u metsu fu metsu
餘國有眾生|yo koku u shu·jo
恭敬信樂者|ku·gyo shin·gyo sha
我復於彼中|ga bu o hi chu
為說無上法|i setsu mu·jo ho
汝等不聞此|nyo·to fu mon shi
但謂我滅度|tan ni ga metsu·do
我見諸眾生|ga ken sho shu·jo
沒在於苦惱|motsu zai o ku·no
故不為現身|ko fu i gen shin
令其生渴仰|ryo go sho katsu·go
因其心戀慕|in go shin ren·bo
乃出為說法|nai shutsu i sep·po
神通力如是|jin·zu riki nyo·ze
於阿僧祇劫|o a·so·gi ko
常在靈鷲山|jo zai ryo·ju sen
及餘諸住處|gyu yo sho ju·sho
眾生見劫盡|shu·jo ken ko jin
大火所燒時|dai ka sho sho ji
我此土安隱|ga shi do an·non
天人常充滿|ten nin jo ju·man
園林諸堂閣|on·rin sho do·kaku
種種寶莊嚴|shu·ju ho sho·gon
寶樹多花菓|ho·ju ta ke ka
眾生所遊樂|shu·jo sho yu·raku
諸天擊天鼓|sho ten gyaku ten ku
常作眾伎樂|jo sa shu gi·gaku
雨曼陀羅花|u man·da·ra ke
散佛及大眾|san butsu gyu dai·shu
我淨土不毀|ga jo·do fu ki
而眾見燒盡|ni shu ken sho jin
憂怖諸苦惱|u·fu sho ku·no
如是悉充滿|nyo·ze shitsu ju·man
是諸罪眾生|ze sho zai shu·jo
以惡業因緣|i aku·go in·nen
過阿僧祇劫|ka a·so·gi ko
不聞三寶名|fu mon san·bo myo
諸有修功德|sho u shu ku·doku
柔和質直者|nyu·wa shichi·jiki sha
則皆見我身|sok·kai ken ga shin
在此而說法|zai shi ni sep·po
或時為此眾|waku ji i shi shu
說佛壽無量|setsu butsu ju mu·ryo
久乃見佛者|ku nai ken bus·sha
為說佛難值|i setsu butsu nan·chi
我智力如是|ga chi·riki nyo·ze
慧光照無量|e·ko sho mu·ryo
壽命無數劫|ju·myo mu·shu ko
久修業所得|ku shu go sho toku
汝等有智者|nyo·to u chi sha
勿於此生疑|mot·to shi sho gi
當斷令永盡|to dan ryo yo jin
佛語實不虛|butsu·go jip·pu ko
如醫善方便|nyo i zen ho·ben
為治狂子故|i ji o·shi ko
實在而言死|jitsu zai ni gon shi
無能說虛妄|mu no sek·ko mo
我亦為世父|ga yaku i se fu
救諸苦患者|ku sho ku gen sha
為凡夫顛倒|i bon·bu ten·do
實在而言滅|jitsu zai ni gon metsu
以常見我故|i jo ken ga ko
而生憍恣心|ni sho kyo·shi shin
放逸著五欲|ho·itsu jaku go yoku
墮於惡道中|da o aku·do chu
我常知眾生|ga jo chi shu·jo
行道不行道|gyo·do fu gyo·do
隨所應可度|zui sho o ka do
為說種種法|i ses·shu·ju ho
每自作是意|mai ji sa ze i
以何令眾生|i ga ryo shu·jo
得入無上慧|toku nyu mu·jo e
速成就佛身|soku jo·ju bus·shin"""
JIGAGE = [tuple(l.split("|")) for l in JIGAGE.splitlines()]



def cbeta(start: str, lines: int) -> str:
    query = urllib.parse.urlencode({"linehead": start, "before": 0, "after": lines})
    with urllib.request.urlopen(f"{API}?{query}", timeout=30, context=TLS) as resp:
        results = json.load(resp)["results"]
    return PUNCT.sub("", "".join(html.unescape(re.sub(r"<[^>]+>", "", r["html"])) for r in results))


def say(romaji: str) -> str:
    s = romaji.replace("·", "")
    return s[0].upper() + s[1:] + "."


def check(name, pairs, source, file) -> list[str]:
    errors = []
    joined = "".join(z for z, _ in pairs)
    if joined != source:
        errors.append(f"{name}: the Chinese differs from CBETA")
    for z, r in pairs:
        units = re.split(r"[ ·-]", r)
        if len(units) != len(z):
            errors.append(f"{name}: {z} has {len(z)} characters but {len(units)} readings in {r!r}")
    text = (TEXTS / file).read_text(encoding="utf-8")
    for _, r in pairs:
        if say(r) not in text:
            errors.append(f"{name}: {file} doesn't say {say(r)!r}")
    return errors


def main() -> int:
    sys.stdout.reconfigure(encoding="utf-8")
    try:
        hoben = cbeta("T09n0262_p0005b25", 18)
        jigage = cbeta("T09n0262_p0043b12", 56)
    except OSError as err:
        print(f"CBETA unreachable: {err}")
        return 2
    # The fetched lines run on past the passage; the passage must be a prefix.
    hoben = hoben[: hoben.index("如是本末究竟等") + len("如是本末究竟等")]
    jigage = jigage[: jigage.index("速成就佛身") + len("速成就佛身")]
    errors = check("chapter 2", HOBEN, hoben, "lotus-2.txt")
    errors += check("chapter 16", JIGAGE, jigage, "lotus-16-verses.txt")
    for e in errors:
        print("FAIL", e)
    print(f"{len(HOBEN)} + {len(JIGAGE)} phrases checked, {len(errors)} problems")
    return 1 if errors else 0


if __name__ == "__main__":
    sys.exit(main())
