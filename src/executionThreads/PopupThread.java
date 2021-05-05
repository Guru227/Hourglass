package executionThreads;

class PopupThread extends Thread {
	Lock lock;
	PopupWindow myPopupWindow;
	//this flag ensures correct starting order of execution and no consecutive execution
		//The latter is ensured by comparing this flag with the flag set in the lock
	boolean flagOfOrder = true;
	
	public PopupThread(Lock l) {
		this.lock = l;
		this.myPopupWindow = new PopupWindow();
		System.out.println("Popup Window has been created");
	}
	public void run() {
		System.out.println("Popup Thread is running");
		while(true) {
			try {
				lock.lock(flagOfOrder);
			} catch (InterruptedException e) {
				e.printStackTrace();
			}
			myPopupWindow.openWindow();
			lock.unlockAndSet(flagOfOrder);
		}
	}
}
