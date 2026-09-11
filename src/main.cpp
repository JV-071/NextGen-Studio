#include "mainwindow.h"
#include <QApplication>
#include <QFile>
#include <QSettings>
#include <QTimer>
#include <QTextStream>
int main(int argc,char** argv){
    QApplication app(argc,argv);
    app.setApplicationName("NextGen Studio");app.setOrganizationName("NextGen");
    app.setApplicationVersion("0.1.0");app.setStyle("Fusion");
    QFile theme(":/resources/theme.qss");if(theme.open(QIODevice::ReadOnly))app.setStyleSheet(QString::fromUtf8(theme.readAll()));
    const bool smoke=app.arguments().contains("--smoke-test");
    if(smoke){QSettings::setDefaultFormat(QSettings::IniFormat);app.setOrganizationName("NextGenTests");}
    studio::MainWindow window;
    if(smoke){QString error;const bool ok=window.smokeTest(error);window.show();app.processEvents();
        if(app.arguments().contains("--screenshot")){const int at=app.arguments().indexOf("--screenshot");if(at+1<app.arguments().size())window.grab().save(app.arguments()[at+1]);}
        QTextStream(stdout)<<(ok?"Desktop smoke passed\n":error+"\n");return ok?0:1;}
    if(argc>1)window.openFile(QString::fromLocal8Bit(argv[1]));else window.openExample();
    window.show();return app.exec();
}
