#include "document.h"
#include <QFile>
#include <QFileInfo>
#include <QSaveFile>
#include <QStringDecoder>
#include <QRegularExpression>
#include <QSet>

namespace studio {
namespace {
constexpr ushort cp1252[] = {0x20ac,0x81,0x201a,0x192,0x201e,0x2026,0x2020,0x2021,
0x2c6,0x2030,0x160,0x2039,0x152,0x8d,0x17d,0x8f,0x90,0x2018,0x2019,0x201c,
0x201d,0x2022,0x2013,0x2014,0x2dc,0x2122,0x161,0x203a,0x153,0x9d,0x17e,0x178};
int indentOf(const QByteArray& s) { int n=0; while(n<s.size() && s[n]==' ') ++n; return n; }
bool atomicWrite(const QString& path, const QByteArray& data, QString& error) {
    QSaveFile file(path);
    file.setDirectWriteFallback(false);
    if (!file.open(QIODevice::WriteOnly) || file.write(data)!=data.size() || !file.commit()) {
        error = file.errorString(); return false;
    }
    return true;
}
}
QString Document::decode(const QByteArray& s) const {
    if (utf8_) return QString::fromUtf8(s);
    QString out; out.reserve(s.size());
    for (unsigned char c:s) out += QChar(c>=0x80 && c<=0x9f ? cp1252[c-0x80] : c);
    return out;
}
QByteArray Document::encode(const QString& s, bool& ok) const {
    ok=true; if(utf8_) return s.toUtf8();
    QByteArray out; out.reserve(s.size());
    for(QChar c:s) {
        const auto u=c.unicode();
        if(u<0x80 || (u>=0xa0 && u<=0xff)) out+=char(u);
        else {
            bool found=false;
            for(int i=0;i<32;++i) if(cp1252[i]==u) { out+=char(i+0x80); found=true; break; }
            if(!found) {ok=false;return {};}
        }
    }
    return out;
}
bool Document::load(const QString& path, QString& error) {
    QFile file(path);
    if(!file.open(QIODevice::ReadOnly)) {error=file.errorString();return false;}
    if(file.size()>MaxBytes) {error="Arquivo excede o limite de 8 MiB.";return false;}
    const auto data=file.readAll();
    if(file.error()!=QFileDevice::NoError) {error=file.errorString();return false;}
    if(!parse(data,error)) return false;
    path_=QFileInfo(path).canonicalFilePath(); savedBytes_=data; return true;
}
bool Document::parse(const QByteArray& bytes, QString& error) {
    if(bytes.size()>MaxBytes || bytes.contains('\0')) {error="Arquivo binário ou maior que 8 MiB.";return false;}
    QByteArray data=bytes;
    const bool bom=data.startsWith("\xef\xbb\xbf");
    if(bom) data.remove(0,3);
    QStringDecoder decoder(QStringDecoder::Utf8); const QString checked=decoder(data); Q_UNUSED(checked);
    const bool validUtf8=!decoder.hasError();
    if(bom && !validUtf8) {error="UTF-8 inválido com BOM.";return false;}
    utf8_=validUtf8;
    bom_=bom ? QByteArray("\xef\xbb\xbf") : QByteArray();
    lines_.clear();
    qsizetype start=0;
    while(start<data.size()) {
        const auto lf=data.indexOf('\n',start);
        if(lf<0) {lines_.push_back({data.mid(start),{}});break;}
        const bool cr=lf>start && data[lf-1]=='\r';
        lines_.push_back({data.mid(start,lf-start-(cr?1:0)),cr?QByteArray("\r\n"):QByteArray("\n")});
        start=lf+1;
    }
    index(); return true;
}
QByteArray Document::bytes() const {
    QByteArray out=bom_; for(const auto& line:lines_) {out+=line.content;out+=line.ending;} return out;
}
void Document::index() {
    nodes_.clear();issues_.clear();QVector<int> stack;int blockIndent=-1;QSet<QString> ids;
    for(int i=0;i<lines_.size();++i) {
        const auto& raw=lines_[i].content; const auto trimmed=raw.trimmed(); const int ind=indentOf(raw);
        if(trimmed.isEmpty() || trimmed.startsWith('#') || trimmed.startsWith("//")) continue;
        if(blockIndent>=0 && ind>blockIndent) continue;
        blockIndent=-1;
        if(raw.mid(ind).startsWith('\t'))
            issues_.push_back({i+1,"Indentação com tabulação: edição estrutural desativada."});
        while(!stack.isEmpty() && nodes_[stack.last()].indent>=ind) {nodes_[stack.last()].end=i;stack.removeLast();}
        const auto colon=trimmed.indexOf(':');
        if(colon>=0) {
            if(stack.isEmpty()) {issues_.push_back({i+1,"Propriedade sem elemento pai."});continue;}
            const auto key=decode(trimmed.left(colon)).trimmed();
            const auto val=decode(trimmed.mid(colon+1)).trimmed();
            const bool multi=val=="|" || val=="|-" || val=="|+";
            nodes_[stack.last()].properties.push_back({i,key,val,multi});
            if(key=="id") {
                // IDs may legitimately repeat across distinct templates: report, never rewrite.
                if(ids.contains(val)) issues_.push_back({i+1,"ID repetido no documento: "+val});
                ids.insert(val);
            }
            if(multi) blockIndent=ind;
            else if(val.isEmpty()) {
                const int idx=nodes_.size();
                nodes_.push_back({i,int(lines_.size()),stack.last(),ind,key,false,{}});
                stack.push_back(idx);
            }
        } else {
            const auto name=decode(trimmed);
            const int idx=nodes_.size();
            nodes_.push_back({i,int(lines_.size()),stack.isEmpty()?-1:stack.last(),ind,name,!name.startsWith('$'),{}});
            stack.push_back(idx);
        }
    }
}
QString Document::value(int node,const QString& key,const QString& fallback) const {
    if(node<0||node>=nodes_.size()) return fallback;
    for(const auto& p:nodes_[node].properties) if(p.key==key) return p.value;
    return fallback;
}
bool Document::setProperty(int node,const QString& key,const QString& val,QString& error) {
    static const QRegularExpression validKey("^[!@*]?[A-Za-z_][A-Za-z0-9_.-]*$");
    if(node<0||node>=nodes_.size()||!validKey.match(key).hasMatch()||val.contains('\n')||val.contains('\r')||val.contains(QChar(0))) {
        error="Propriedade ou seleção inválida.";return false;
    }
    for(const auto& issue:issues_) if(issue.message.contains("tabulação")) {error=issue.message;return false;}
    if(bytes().size()+val.size()*4+key.size()+64>MaxBytes) {error="Documento excederia o limite de 8 MiB.";return false;}
    if(val.trimmed().isEmpty()) {error="Use o editor de código para criar blocos vazios.";return false;}
    bool ok=false;const auto data=encode(val,ok);
    if(!ok) {error="Este caractere não pode ser salvo em Windows-1252.";return false;}
    const auto n=nodes_[node];
    int matches=0;for(const auto& p:n.properties) if(p.key==key) ++matches;
    if(matches>1) {error="Propriedade duplicada: edite no código para resolver a ambiguidade.";return false;}
    for(const auto& p:n.properties) if(p.key==key) {
        if(p.multiline || p.value.isEmpty()) {error="Bloco composto: use a aba Código.";return false;}
        auto& line=lines_[p.line].content;
        const auto colon=line.indexOf(':');qsizetype v=colon+1;
        while(v<line.size() && (line[v]==' '||line[v]=='\t')) ++v;
        line=line.left(v)+data;index();return true;
    }
    if(val.isEmpty()) {error="Uma propriedade nova precisa de valor.";return false;}
    const auto ending=lines_.isEmpty()?QByteArray("\n"):(lines_[n.line].ending.isEmpty()?QByteArray("\n"):lines_[n.line].ending);
    if(lines_[n.line].ending.isEmpty()) lines_[n.line].ending=ending;
    lines_.insert(n.line+1,{QByteArray(n.indent+2,' ')+key.toUtf8()+": "+data,ending});index();return true;
}
bool Document::insertWidget(int parent,const QString& type,const QString& id,QString& error) {
    if(bytes().size()+type.size()+id.size()+256>MaxBytes) {error="Documento excederia o limite de 8 MiB.";return false;}
    static const QRegularExpression valid("^[A-Za-z_][A-Za-z0-9_]*$");
    if(!valid.match(type).hasMatch()||!valid.match(id).hasMatch()||parent>=nodes_.size()||parent< -1) {error="Tipo ou ID inválido.";return false;}
    for(int i=0;i<nodes_.size();++i) if(value(i,"id")==id) {error="ID já existe.";return false;}
    for(const auto& issue:issues_) if(issue.message.contains("tabulação")) {error=issue.message;return false;}
    if(parent>=0&&!nodes_[parent].widget) {error="Selecione um widget como pai.";return false;}
    int at=parent<0?lines_.size():nodes_[parent].end;
    int ind=parent<0?0:nodes_[parent].indent+2;
    QByteArray eol="\n";for(const auto& l:lines_) if(!l.ending.isEmpty()) {eol=l.ending;break;}
    if(at>0 && lines_[at-1].ending.isEmpty()) lines_[at-1].ending=eol;
    QVector<Line> added{{QByteArray(ind,' ')+type.toUtf8(),eol},
        {QByteArray(ind+2,' ')+"id: "+id.toUtf8(),eol},
        {QByteArray(ind+2,' ')+"size: 120 32",eol}};
    for(int i=0;i<added.size();++i) lines_.insert(at+i,added[i]);
    index();return true;
}
bool Document::removeNode(int node,QString& error) {
    if(node<0||node>=nodes_.size()||!nodes_[node].widget) {error="Selecione um elemento.";return false;}
    for(const auto& issue:issues_) if(issue.message.contains("tabulação")) {error=issue.message;return false;}
    const auto n=nodes_[node];
    int end=n.end;
    // Keep comments and whitespace separating the next sibling.
    while(end>n.line+1) {const auto t=lines_[end-1].content.trimmed();if(t.isEmpty()||t.startsWith("#")||t.startsWith("//")) --end;else break;}
    lines_.remove(n.line,end-n.line);index();return true;
}
QString Document::text() const {auto data=bytes();if(!bom_.isEmpty()) data.remove(0,bom_.size());return decode(data);}
bool Document::replaceText(const QString& s,QString& error) {
    bool ok;auto data=encode(s,ok);
    if(!ok) {error="Texto não representável em Windows-1252.";return false;}
    return parse(bom_+data,error);
}
bool Document::diskChanged() const {
    if(path_.isEmpty()) return false;
    QFile f(path_);return !f.open(QIODevice::ReadOnly)||f.size()>MaxBytes||f.readAll()!=savedBytes_;
}
bool Document::save(const QString& path,bool overwrite,QString& error) {
    const QFileInfo info(path);
    if(info.isSymLink()) {error="Salvamento em link simbólico não é permitido.";return false;}
    const bool same=!path_.isEmpty() && info.absoluteFilePath()==QFileInfo(path_).absoluteFilePath();
    if(same && diskChanged()) {error="O arquivo mudou fora do editor. Reabra ou salve uma cópia.";return false;}
    if(!same && info.exists() && !overwrite) {error="O destino já existe.";return false;}
    if(info.exists()) {
        QFile original(path);
        if(!original.open(QIODevice::ReadOnly)||original.size()>MaxBytes) {error="Não foi possível preparar o backup.";return false;}
        if(!atomicWrite(path+".bak",original.readAll(),error)) return false;
    }
    const auto data=bytes();
    if(!atomicWrite(path,data,error)) return false;
    path_=QFileInfo(path).canonicalFilePath();savedBytes_=data;return true;
}
}
