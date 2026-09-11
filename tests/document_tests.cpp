#include "document.h"
#include <QCoreApplication>
#include <QTemporaryDir>
#include <QFile>
#include <QTextStream>
#include <cstdlib>
using studio::Document;
static int checks=0;
static void require(bool ok,const char* message){++checks;if(!ok){QTextStream(stderr)<<"FAIL: "<<message<<"\n";std::exit(1);}}
static QByteArray read(const QString& path){QFile f(path);require(f.open(QIODevice::ReadOnly),"read file");return f.readAll();}
static void write(const QString& path,const QByteArray& bytes){QFile f(path);require(f.open(QIODevice::WriteOnly),"write file");require(f.write(bytes)==bytes.size(),"write all");}
int main(int argc,char** argv){
    QCoreApplication app(argc,argv);QString error;Document d;
    const auto original=QByteArray::fromHex("efbbbf")+QByteArray("# keep\r\nMainWindow\r\n  id: main\r\n  text: Inventario\r\n  @onClick: |\r\n    if a then\r\n      foo('x:y')\r\n    end\r\n  $hover:\r\n    color: red\r\n  Button\r\n    id: close\r\n    text: Fechar");
    require(d.parse(original,error),"parse BOM and multiline");
    require(d.bytes()==original,"byte exact roundtrip");
    require(d.nodes().size()==3,"script lines are not widgets");
    require(!d.nodes()[1].widget,"state is not a widget");
    require(d.setProperty(2,"text","Sair",error),"edit nested property");
    auto expected=original;expected.replace("text: Fechar","text: Sair");
    require(d.bytes()==expected,"minimal patch preserves BOM comments CRLF no final newline");
    require(!d.setProperty(0,"@onClick","changed",error),"multiline property cannot be destroyed by inspector");
    require(!d.setProperty(0,"a\nb","x",error),"invalid key");
    require(!d.setProperty(0,"text","x\ny",error),"invalid scalar");
    require(d.insertWidget(0,"Label","newLabel",error),"insert child");
    require(d.value(3,"id")=="newLabel","new child indexed");
    require(!d.insertWidget(0,"Button","newLabel",error),"duplicate id rejected");
    require(d.removeNode(3,error),"remove child");
    require(d.bytes().startsWith(QByteArray::fromHex("efbbbf")),"BOM survives structural changes");

    QByteArray legacy("Label\n  text: Caf");legacy+=char(0xe9);legacy+="\n";
    require(d.parse(legacy,error),"parse CP1252");
    require(d.encoding()=="Windows-1252","detect CP1252");
    require(d.bytes()==legacy,"CP1252 roundtrip");
    require(d.setProperty(0,"text",QString::fromUtf8("Ação"),error),"encode CP1252");
    auto before=d.bytes();
    require(!d.setProperty(0,"text",QString::fromUtf8("😀"),error),"reject unrepresentable text");
    require(d.bytes()==before,"failed mutation keeps document");
    require(!d.parse(QByteArray::fromHex("efbbbfff"),error),"reject corrupt BOM");
    require(d.bytes()==before,"failed parse is transactional");
    require(d.encoding()=="Windows-1252","failed parse keeps encoding");

    require(d.parse("Panel\n  layout:\n    type: verticalBox\n  Button\n    text: One\n  Button\n    text: Two\n",error),"layout parse");
    require(d.nodes().size()==4&&!d.nodes()[1].widget,"layout group");
    require(d.nodes()[2].parent==0&&d.nodes()[3].parent==0,"siblings parent");
    require(d.removeNode(2,error),"remove sibling");
    require(d.bytes().contains("text: Two")&&!d.bytes().contains("text: One"),"preserve next sibling");
    require(d.parse("Label\n  text: a\n  text: b\n",error),"duplicate scalar parse");
    require(!d.setProperty(0,"text","c",error),"ambiguous mutation rejected");
    require(!d.parse(QByteArray("a\0b",3),error),"binary rejected");
    require(!d.parse(QByteArray(Document::MaxBytes+1,'x'),error),"oversize rejected");
    require(d.parse("Label\n\ttext: bad\n",error),"tabs diagnosed");
    require(!d.issues().isEmpty(),"tabs diagnostic");
    require(!d.insertWidget(0,"Button","b",error),"tabs prevent structure edit");

    QTemporaryDir temp;require(temp.isValid(),"temporary directory");
    auto target=temp.filePath("interface.otui");
    require(d.parse("Label\n  text: original\n",error),"save sample");
    require(d.save(target,false,error),"initial atomic save");
    Document copy;require(copy.load(target,error),"load saved document");
    require(copy.setProperty(0,"text","edited",error),"change saved");
    require(copy.save(target,false,error),"atomic replacement");
    require(read(target+".bak")=="Label\n  text: original\n","backup exact");
    const auto changed=QByteArray("Label\n  text: external\n");write(target,changed);
    require(copy.diskChanged(),"external change detected");
    require(!copy.save(target,true,error),"refuse external overwrite even with overwrite flag");
    require(read(target)==changed,"external file preserved");
    require(!d.save(target,false,error),"refuse conflicting saved file");
    require(copy.save(temp.filePath("copy.otui"),false,error),"save conflict as a new copy");
    QTextStream(stdout)<<"Passed "<<checks<<" document integrity checks\n";return 0;
}
